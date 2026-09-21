use std::ffi::c_void;

pub mod engine;

pub use engine::Engine;

pub type ReplEvalFn = unsafe extern "C" fn(*mut c_void);
