use anyhow::{Context, Result};
use temp_rs_compiler::{
    driver::compile_cdylib,
    generator::generate_source,
    parser::{InputKind, classify_input, is_persistent_binding},
};
use temp_rs_dylib_loader::loader::MiniLoader;
use tempfile::TempDir;

use crate::ReplEvalFn;

pub struct Engine {
    scratch_dir: TempDir,
    /// Previous eval library. Dropped before the next compile overwrites it.
    loader: Option<MiniLoader>,
    item_history: Vec<String>,
    binding_history: Vec<String>,
    counter: usize,
}

impl Engine {
    pub fn new() -> Result<Self> {
        MiniLoader::preload_runtime()?;
        let scratch_dir = TempDir::new().context("failed to create REPL scratch directory")?;
        let mut engine = Self {
            scratch_dir,
            loader: None,
            item_history: Vec::new(),
            binding_history: Vec::new(),
            counter: 0,
        };
        // Map the stable dylib once so the first real eval reuses dyld's cache.
        engine.warm()?;
        Ok(engine)
    }

    fn warm(&mut self) -> Result<()> {
        let source = r#"#[no_mangle]
pub extern "C" fn __repl_eval(_ctx: *mut std::ffi::c_void) {}
"#;
        let artifact = compile_cdylib(source, self.counter, self.scratch_dir.path())?;
        self.counter += 1;
        let loader = MiniLoader::open(&artifact.dylib_path)?;
        drop(loader);
        Ok(())
    }

    pub fn eval(&mut self, snippet: &str) -> Result<()> {
        let kind = classify_input(snippet);

        let source = generate_source(&kind, &self.item_history, &self.binding_history);

        // The published dylib keeps a stable inode. Unmap it before that file
        // is overwritten, or dyld will keep executing the previous mapping.
        self.loader.take();

        let artifact = compile_cdylib(&source, self.counter, self.scratch_dir.path())?;
        self.counter += 1;

        let loader = MiniLoader::open(&artifact.dylib_path)?;

        unsafe {
            let eval_fn = loader.get_symbol::<ReplEvalFn>("__repl_eval")?;
            eval_fn(std::ptr::null_mut());
        }
        self.loader = Some(loader);

        match kind {
            InputKind::Item(code) => self.item_history.push(code),
            InputKind::Statement(code) | InputKind::Expression(code)
                if is_persistent_binding(&code) =>
            {
                self.binding_history.push(code);
            }
            _ => {}
        }

        Ok(())
    }
}
