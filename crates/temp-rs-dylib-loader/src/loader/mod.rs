// Not used, using the bindings provided by libc
// pub mod c_ffi;

use std::{
    ffi::{CStr, CString, c_char, c_void},
    mem::{size_of, transmute_copy},
    os::unix::ffi::OsStrExt,
    path::Path,
};

#[cfg(target_os = "macos")]
use std::{fs, path::PathBuf, process::Command, sync::OnceLock};

use anyhow::{Context, Result, bail};
use libc::{RTLD_GLOBAL, RTLD_LAZY, dlclose, dlerror, dlopen, dlsym};

#[cfg(target_os = "macos")]
use libc::RTLD_NOW;

/// Handle of the preloaded `libstd`, kept for the life of the process.
#[cfg(target_os = "macos")]
static RUNTIME: OnceLock<usize> = OnceLock::new();

pub struct MiniLoader {
    handle: *mut c_void,
}

impl Drop for MiniLoader {
    fn drop(&mut self) {
        unsafe {
            dlclose(self.handle);
        }
    }
}

impl MiniLoader {
    /// Load the toolchain `libstd` into the global namespace.
    ///
    /// Eval libraries are linked with undefined std symbols. One preloaded
    /// `libstd` satisfies every later `dlopen` without dyld rebinding it.
    pub fn preload_runtime() -> Result<()> {
        #[cfg(not(target_os = "macos"))]
        {
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            if RUNTIME.get().is_some() {
                return Ok(());
            }

            let libdir = Self::target_libdir()?;
            let std_path = fs::read_dir(&libdir)
                .with_context(|| format!("failed to read {}", libdir.display()))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("libstd-") && name.ends_with(".dylib"))
                })
                .context("libstd dylib not found in the rustc sysroot")?;

            let c_path = CString::new(std_path.as_os_str().as_bytes())
                .with_context(|| format!("invalid libstd path: {}", std_path.display()))?;

            unsafe {
                dlerror();
            }
            let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
            if handle.is_null() {
                bail!(
                    "failed to load runtime '{}': {}",
                    std_path.display(),
                    Self::last_error()
                );
            }

            let _ = RUNTIME.set(handle as usize);
            Ok(())
        }
    }

    pub fn open(path: &Path) -> Result<Self> {
        let c_path = CString::new(path.as_os_str().as_bytes())
            .with_context(|| format!("invalid library path: {}", path.display()))?;

        unsafe {
            dlerror();
        }

        let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_LAZY | RTLD_GLOBAL) };

        if handle.is_null() {
            bail!(
                "failed to load '{}': {}",
                path.display(),
                Self::last_error()
            );
        }

        Ok(Self { handle })
    }

    /// Looks up `name` in the loaded library and reinterprets it as `T`.
    ///
    /// # Safety
    ///
    /// `T` must be the type of the exported symbol `name`. The returned value
    /// is valid only while this loader keeps the library mapped.
    pub unsafe fn get_symbol<T>(&self, name: &str) -> Result<T> {
        if size_of::<T>() != size_of::<*mut c_void>() {
            bail!("requested symbol type is not pointer-sized");
        }

        let c_name = CString::new(name).with_context(|| format!("invalid symbol name: {name}"))?;

        unsafe {
            dlerror();
        }

        let symbol_ptr = unsafe { dlsym(self.handle, c_name.as_ptr()) };

        let err = unsafe { dlerror() };
        if !err.is_null() {
            bail!("symbol `{name}` not found: {}", Self::error_message(err));
        }
        if symbol_ptr.is_null() {
            bail!("symbol `{name}` resolved to a null pointer");
        }

        let func: T = unsafe { transmute_copy(&symbol_ptr) };
        Ok(func)
    }

    fn last_error() -> String {
        let err_ptr = unsafe { dlerror() };
        Self::error_message(err_ptr)
    }

    #[cfg(target_os = "macos")]
    fn target_libdir() -> Result<PathBuf> {
        let output = Command::new("rustc")
            .arg("--print")
            .arg("target-libdir")
            .output()
            .context("failed to spawn rustc")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("rustc --print target-libdir failed: {stderr}");
        }
        let dir = String::from_utf8(output.stdout).context("rustc target-libdir was not utf-8")?;
        Ok(PathBuf::from(dir.trim()))
    }

    fn error_message(err_ptr: *mut c_char) -> String {
        if err_ptr.is_null() {
            "Unknown dlerror".to_string()
        } else {
            unsafe { CStr::from_ptr(err_ptr) }
                .to_string_lossy()
                .into_owned()
        }
    }
}
