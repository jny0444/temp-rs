use anyhow::{Context, Result};
use temp_rs_compiler::{
    driver::{compile_cdylib, compile_cdylib_mapped},
    generator::generate_source,
    parser::{InputKind, classify_input, is_persistent_binding, item_keys},
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

        let replaced_items;
        let items = match &kind {
            InputKind::Item(code) => {
                replaced_items = items_without_conflicts(&self.item_history, code);
                replaced_items.as_slice()
            }
            _ => self.item_history.as_slice(),
        };

        let generated = generate_source(&kind, items, &self.binding_history);

        // The published dylib keeps a stable inode. Unmap it before that file
        // is overwritten, or dyld will keep executing the previous mapping.
        self.loader.take();

        let artifact = compile_cdylib_mapped(
            &generated.source,
            self.counter,
            self.scratch_dir.path(),
            generated.snippet_start_line,
        )?;
        self.counter += 1;

        let loader = MiniLoader::open(&artifact.dylib_path)?;

        unsafe {
            let eval_fn = loader.get_symbol::<ReplEvalFn>("__repl_eval")?;
            eval_fn(std::ptr::null_mut());
        }
        self.loader = Some(loader);

        match kind {
            InputKind::Item(code) => upsert_item(&mut self.item_history, code),
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

fn items_without_conflicts(history: &[String], new: &str) -> Vec<String> {
    let keys = item_keys(new);
    if keys.is_empty() {
        return history.to_vec();
    }
    history
        .iter()
        .filter(|old| item_keys(old).is_disjoint(&keys))
        .cloned()
        .collect()
}

fn upsert_item(history: &mut Vec<String>, new: String) {
    let keys = item_keys(&new);
    if !keys.is_empty() {
        history.retain(|old| item_keys(old).is_disjoint(&keys));
    }
    history.push(new);
}
