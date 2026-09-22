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
        emit_object(&src_file, &obj_file)?;
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
            .arg("-C")
            .arg("prefer-dynamic")
            .arg("-o")
            .arg(&dylib_file)
            .output()
            .context("failed to spawn rustc")?;

        if !output.status.success() {
            bail!("{}", rustc_diagnostics(&output));
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
        .arg("-C")
        .arg("codegen-units=1")
        .arg("--cap-lints")
        .arg("allow")
        .arg(src_file);
    command
}

#[cfg(target_os = "macos")]
fn emit_object(src_file: &Path, obj_file: &Path) -> Result<()> {
    let output = rustc_command(src_file)
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--emit=obj")
        .arg("-o")
        .arg(obj_file)
        .output()
        .context("failed to spawn rustc")?;

    if !output.status.success() {
        bail!("{}", rustc_diagnostics(&output));
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
