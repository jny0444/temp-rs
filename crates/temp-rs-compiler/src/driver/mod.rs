use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(target_os = "macos")]
use std::io::Write;

use anyhow::{Context, Result, bail};

#[derive(Debug)]
pub struct Artifact {
    pub dylib_path: PathBuf,
}

pub fn dylib_filename(counter: usize) -> String {
    if cfg!(target_os = "windows") {
        format!("libeval_{counter}.dll")
    } else if cfg!(target_os = "macos") {
        format!("libeval_{counter}.dylib")
    } else {
        format!("libeval_{counter}.so")
    }
}

pub fn compile_cdylib(source_code: &str, counter: usize, target_dir: &Path) -> Result<Artifact> {
    compile_cdylib_mapped(source_code, counter, target_dir, 1)
}

/// Like [`compile_cdylib`], but rustc spans are rewritten so line 1 is the
/// user's snippet (`snippet_start_line` in the generated file, 1-based).
pub fn compile_cdylib_mapped(
    source_code: &str,
    counter: usize,
    target_dir: &Path,
    snippet_start_line: usize,
) -> Result<Artifact> {
    fs::create_dir_all(target_dir).context("compiler I/O error")?;

    let src_file = target_dir.join(format!("eval_{counter}.rs"));

    fs::write(&src_file, source_code).context("compiler I/O error")?;

    // macOS spends most of an eval inside dyld, building a closure for each new
    // inode. Emit an object, link it without a libstd dependency, and publish
    // the bytes into one stable file so later loads reuse that closure.
    #[cfg(target_os = "macos")]
    {
        let obj_file = target_dir.join(format!("eval_{counter}.o"));
        let linked_file = target_dir.join(format!("link_{counter}.dylib"));
        let published = target_dir.join(dylib_filename(0));
        emit_object(&src_file, &obj_file, snippet_start_line)?;
        link_macos_dylib(&obj_file, &linked_file)?;
        publish_in_place(&linked_file, &published)?;
        let _ = fs::remove_file(&linked_file);
        Ok(Artifact {
            dylib_path: published,
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let dylib_file = target_dir.join(dylib_filename(counter));
        let output = rustc_command(&src_file)
            .arg("--crate-type")
            .arg("cdylib")
            .arg("-o")
            .arg(&dylib_file)
            .output()
            .context("failed to spawn rustc")?;

        if !output.status.success() {
            bail!(
                "{}",
                remap_diagnostics(&rustc_diagnostics(&output), &src_file, snippet_start_line)
            );
        }

        Ok(Artifact {
            dylib_path: dylib_file,
        })
    }
}

fn rustc_command(src_file: &Path) -> Command {
    let mut command = Command::new("rustc");
    command
        .arg("--edition=2021")
        .arg("--crate-name")
        .arg("repl_eval")
        .arg("-C")
        .arg("opt-level=0")
        .arg("-C")
        .arg("debuginfo=0")
        .arg("--cap-lints")
        .arg("allow")
        .arg(src_file);
    command
}

#[cfg(target_os = "macos")]
fn emit_object(src_file: &Path, obj_file: &Path, snippet_start_line: usize) -> Result<()> {
    let output = rustc_command(src_file)
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--emit=obj")
        .arg("-o")
        .arg(obj_file)
        .output()
        .context("failed to spawn rustc")?;

    if !output.status.success() {
        bail!(
            "{}",
            remap_diagnostics(&rustc_diagnostics(&output), src_file, snippet_start_line)
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn link_macos_dylib(obj_file: &Path, dylib_file: &Path) -> Result<()> {
    let builtins = compiler_builtins_rlib()?;
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };

    let output = Command::new("cc")
        .arg(obj_file)
        .arg(&builtins)
        .arg("-lSystem")
        .arg("-lc")
        .arg("-lm")
        .arg("-arch")
        .arg(arch)
        .arg("-dynamiclib")
        .arg("-nodefaultlibs")
        .arg("-undefined")
        .arg("dynamic_lookup")
        .arg("-o")
        .arg(dylib_file)
        .output()
        .context("failed to spawn cc")?;

    if !output.status.success() {
        bail!("{}", rustc_diagnostics(&output));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn compiler_builtins_rlib() -> Result<PathBuf> {
    let libdir = target_libdir()?;
    fs::read_dir(&libdir)
        .with_context(|| format!("failed to read {}", libdir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("libcompiler_builtins-") && name.ends_with(".rlib")
                })
        })
        .context("compiler_builtins rlib not found in the rustc sysroot")
}

#[cfg(target_os = "macos")]
fn target_libdir() -> Result<PathBuf> {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("target-libdir")
        .output()
        .context("failed to spawn rustc")?;
    if !output.status.success() {
        bail!("{}", rustc_diagnostics(&output));
    }
    let dir = String::from_utf8(output.stdout).context("rustc target-libdir was not utf-8")?;
    Ok(PathBuf::from(dir.trim()))
}

/// Replace `dest` without changing its inode once it exists.
///
/// dyld caches the closure of a dylib by inode. Unlinking and creating a new
/// file misses that cache and makes every eval pay to map the library again.
#[cfg(target_os = "macos")]
fn publish_in_place(src: &Path, dest: &Path) -> Result<()> {
    let bytes = fs::read(src).context("compiler I/O error")?;
    if dest.exists() {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(dest)
            .context("compiler I/O error")?;
        file.write_all(&bytes).context("compiler I/O error")?;
    } else {
        fs::write(dest, bytes).context("compiler I/O error")?;
    }
    Ok(())
}

fn rustc_diagnostics(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    match (stderr.is_empty(), stdout.is_empty()) {
        (false, true) => stderr.into_owned(),
        (true, false) => stdout.into_owned(),
        (false, false) => format!("{stderr}{stdout}"),
        (true, true) => format!("rustc exited with {}", output.status),
    }
}

/// Rewrite rustc's `eval_N.rs:line:col` spans so they point at the snippet.
fn remap_diagnostics(raw: &str, src_file: &Path, snippet_start_line: usize) -> String {
    let full = src_file.display().to_string();
    let name = src_file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("eval.rs");

    let mut out = String::with_capacity(raw.len());
    let mut in_repl_span = false;

    for (i, line) in raw.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if let Some((text, is_repl)) = remap_span_line(line, &full, name, snippet_start_line) {
            in_repl_span = is_repl;
            out.push_str(&text);
        } else if in_repl_span {
            out.push_str(&remap_gutter_line(line, snippet_start_line));
        } else {
            out.push_str(&line.replace(&full, "<repl>").replace(name, "<repl>"));
        }
    }
    if raw.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn remap_span_line(
    line: &str,
    full: &str,
    name: &str,
    snippet_start_line: usize,
) -> Option<(String, bool)> {
    let marker = if line.contains("--> ") {
        "--> "
    } else if line.contains("::: ") {
        "::: "
    } else {
        return None;
    };
    let (prefix, rest) = line.split_once(marker)?;
    let (file, line_no, col) = split_file_line_col(rest)?;
    let is_repl = file == full || file.ends_with(name);
    if !is_repl {
        return Some((line.to_string(), false));
    }
    let mapped = map_line(line_no, snippet_start_line);
    Some((format!("{prefix}{marker}<repl>:{mapped}:{col}"), true))
}

fn split_file_line_col(rest: &str) -> Option<(&str, usize, &str)> {
    let (file_and_line, col) = rest.rsplit_once(':')?;
    let (file, line) = file_and_line.rsplit_once(':')?;
    let line_no = line.parse().ok()?;
    Some((file, line_no, col))
}

fn remap_gutter_line(line: &str, snippet_start_line: usize) -> String {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    let num_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if num_start == i {
        return line.to_string();
    }
    let num_end = i;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'|' {
        return line.to_string();
    }
    let Ok(n) = line[num_start..num_end].parse::<usize>() else {
        return line.to_string();
    };
    let mapped = map_line(n, snippet_start_line);
    format!(
        "{mapped:>width$}{rest}",
        width = num_end,
        rest = &line[num_end..]
    )
}

fn map_line(generated_line: usize, snippet_start_line: usize) -> usize {
    if generated_line >= snippet_start_line {
        generated_line - snippet_start_line + 1
    } else {
        generated_line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn remap_strips_eval_path_and_shifts_lines() {
        let src = Path::new("/tmp/scratch/eval_3.rs");
        let raw = "\
error[E0308]: mismatched types
 --> /tmp/scratch/eval_3.rs:12:5
  |
12 |     1 + \"a\"
   |     ^
";
        let mapped = remap_diagnostics(raw, src, 12);
        assert!(!mapped.contains("eval_3.rs"), "{mapped}");
        assert!(mapped.contains("<repl>:1:5"), "{mapped}");
        assert!(
            mapped.contains("\n 1 |     1 + \"a\"\n") || mapped.contains("\n1 |     1 + \"a\"\n"),
            "{mapped}"
        );
    }
}
