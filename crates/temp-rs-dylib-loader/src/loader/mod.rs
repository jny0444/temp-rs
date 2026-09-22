// Not used, using the bindings provided by libc
// pub mod c_ffi;

use std::{
    ffi::{CStr, CString, c_char, c_void},
    mem::{size_of, transmute_copy},
    os::unix::ffi::OsStrExt,
    path::Path,
};

use anyhow::{Context, Result, bail};
use libc::{RTLD_GLOBAL, RTLD_LAZY, dlerror, dlopen, dlsym};

pub struct MiniLoader {
    handle: *mut c_void,
}

impl MiniLoader {
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
