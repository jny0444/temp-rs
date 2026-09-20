use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    unsafe fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    unsafe fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    unsafe fn dlclose(handle: *mut c_void) -> c_int;
    unsafe fn dlerror() -> *mut c_char;
}
