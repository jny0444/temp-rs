use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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
    let dylib_file = target_dir.join(dylib_filename(counter));

    fs::write(&src_file, source_code).context("compiler I/O error")?;

    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type")
        .arg("cdylib")
        .arg("-C")
        .arg("opt-level=0")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-C")
        .arg("prefer-dynamic")
        .arg(&src_file)
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
