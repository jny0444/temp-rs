// Not used, using the bindings provided by libc
// pub mod c_ffi;

use std::{
    ffi::{CStr, CString, c_char, c_void},
    io::{Error, ErrorKind, Result},
    mem::{size_of, transmute_copy},
    os::unix::ffi::OsStrExt,
    path::Path,
};

use libc::{RTLD_GLOBAL, RTLD_LAZY, dlerror, dlopen, dlsym};

pub struct MiniLoader {
    handle: *mut c_void,
}

impl MiniLoader {
    pub fn open(path: &Path) -> Result<Self> {
        let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|e| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("CString conversion error: {e}"),
            )
        })?;

        unsafe {
            dlerror();
        }

        let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_LAZY | RTLD_GLOBAL) };

        if handle.is_null() {
            let err_msg = Self::last_error();
            return Err(Error::other(format!("dlopen failed: {err_msg}")));
        }

        Ok(Self { handle })
    }

    pub unsafe fn get_symbol<T>(&self, name: &str) -> Result<T> {
        if size_of::<T>() != size_of::<*mut c_void>() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "symbol type must be pointer-sized",
            ));
        }

        let c_name = CString::new(name).map_err(|e| {
            Error::new(ErrorKind::InvalidInput, format!("Invalid symbol name: {e}"))
        })?;

        unsafe {
            dlerror();
        }

        let symbol_ptr = unsafe { dlsym(self.handle, c_name.as_ptr()) };

        let err = unsafe { dlerror() };
        if !err.is_null() {
            return Err(Error::other(format!(
                "dlsym failed: {}",
                Self::error_message(err)
            )));
        }
        if symbol_ptr.is_null() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Symbol '{name}' resolved to null"),
            ));
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
