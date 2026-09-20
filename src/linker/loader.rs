use std::{
    ffi::{CStr, CString, c_void},
    io::{Error, ErrorKind, Result},
    mem::transmute_copy,
    path::Path,
};

use libc::{RTLD_GLOBAL, RTLD_LAZY, dlclose, dlerror, dlopen, dlsym};

pub struct MiniLoader {
    handle: *mut c_void,
}

impl MiniLoader {
    pub fn open(path: &Path) -> Result<Self> {
        let c_path = CString::new(
            path.to_str()
                .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid path"))?,
        )
        .map_err(|e| {
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
                Self::last_error()
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
        unsafe {
            let err_ptr = dlerror();
            if err_ptr.is_null() {
                "Unknown dlerror".to_string()
            } else {
                CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for MiniLoader {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                dlclose(self.handle);
            }
        }
    }
}
