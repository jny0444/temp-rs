use temp_rs_compiler::driver::{compile_cdylib, dylib_filename};

fn eval_source() -> &'static str {
    r#"
#[no_mangle]
pub extern "C" fn __repl_eval(_ctx: *mut std::ffi::c_void) {}
"#
}

#[test]
fn dylib_filename_matches_platform() {
    let name = dylib_filename(0);
    assert!(
        name.ends_with(".dylib") || name.ends_with(".so") || name.ends_with(".dll"),
        "{name}"
    );
    assert!(name.contains("eval_0"), "{name}");
}

#[test]
fn compile_cdylib_writes_library() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = compile_cdylib(eval_source(), 0, dir.path()).unwrap();
    assert!(artifact.dylib_path.exists());
    assert_eq!(artifact.dylib_path, dir.path().join(dylib_filename(0)));
}

#[cfg(target_os = "macos")]
#[test]
fn repeated_compile_reuses_dylib_inode() {
    use std::os::unix::fs::MetadataExt;

    let dir = tempfile::tempdir().unwrap();
    let first = compile_cdylib(eval_source(), 0, dir.path()).unwrap();
    let inode = std::fs::metadata(&first.dylib_path).unwrap().ino();
    let second = compile_cdylib(eval_source(), 1, dir.path()).unwrap();
    assert_eq!(first.dylib_path, second.dylib_path);
    assert_eq!(inode, std::fs::metadata(&second.dylib_path).unwrap().ino());
}

#[test]
fn compile_cdylib_surfaces_rustc_errors() {
    let dir = tempfile::tempdir().unwrap();
    let err = compile_cdylib("fn broken(", 0, dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("error"), "{msg}");
}
