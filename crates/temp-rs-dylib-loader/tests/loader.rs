use std::{fs, path::PathBuf, process::Command};

use temp_rs_dylib_loader::loader::MiniLoader;

type AddFn = unsafe extern "C" fn(i32, i32) -> i32;

fn fixture_lib() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("add.rs");
    fs::write(
        &src,
        r#"
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .unwrap();

    let lib = dir.path().join(if cfg!(windows) {
        "add.dll"
    } else if cfg!(target_os = "macos") {
        "libadd.dylib"
    } else {
        "libadd.so"
    });

    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=cdylib")
        .arg("-C")
        .arg("opt-level=0")
        .arg("-o")
        .arg(&lib)
        .arg(&src)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    (dir, lib)
}

#[test]
fn open_and_call_symbol() {
    let (_dir, lib) = fixture_lib();
    let loader = MiniLoader::open(&lib).unwrap();
    let add: AddFn = unsafe { loader.get_symbol("add") }.unwrap();
    assert_eq!(unsafe { add(2, 3) }, 5);
}

#[test]
fn missing_symbol_errors() {
    let (_dir, lib) = fixture_lib();
    let loader = MiniLoader::open(&lib).unwrap();
    let err = unsafe { loader.get_symbol::<AddFn>("nope") }.unwrap_err();
    assert!(format!("{err}").contains("nope"), "{err}");
}

#[test]
fn rejects_non_pointer_sized_type() {
    let (_dir, lib) = fixture_lib();
    let loader = MiniLoader::open(&lib).unwrap();
    let err = unsafe { loader.get_symbol::<u8>("add") }.unwrap_err();
    assert!(format!("{err}").contains("pointer-sized"), "{err}");
}

#[test]
fn open_missing_file_errors() {
    let err = MiniLoader::open(std::path::Path::new("/no/such/lib.dylib"))
        .err()
        .expect("opening a missing library should fail");
    assert!(format!("{err}").contains("failed to load"), "{err}");
}
