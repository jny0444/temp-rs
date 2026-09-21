use anyhow::{Context, Result};
use temp_rs_compiler::{
    driver::compile_cdylib,
    generator::generate_source,
    parser::{InputKind, classify_input},
};
use temp_rs_dylib_loader::loader::MiniLoader;
use tempfile::TempDir;

use crate::ReplEvalFn;

pub struct Engine {
    scratch_dir: TempDir,
    loaders: Vec<MiniLoader>,
    item_history: Vec<String>,
    counter: usize,
}

impl Engine {
    pub fn new() -> Result<Self> {
        let scratch_dir = TempDir::new().context("failed to create REPL scratch directory")?;
        Ok(Self {
            scratch_dir,
            loaders: Vec::new(),
            item_history: Vec::new(),
            counter: 0,
        })
    }

    pub fn eval(&mut self, snippet: &str) -> Result<()> {
        let kind = classify_input(snippet);

        let source = generate_source(&kind, &self.item_history);

        let artifact = compile_cdylib(&source, self.counter, self.scratch_dir.path())?;
        self.counter += 1;

        let loader = MiniLoader::open(&artifact.dylib_path)?;
        self.loaders.push(loader);
        let loader = self.loaders.last().unwrap();

        unsafe {
            let eval_fn = loader.get_symbol::<ReplEvalFn>("__repl_eval")?;
            eval_fn(std::ptr::null_mut());
        }

        if let InputKind::Item(code) = kind {
            self.item_history.push(code);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;

    #[test]
    fn eval_expression() {
        let mut engine = Engine::new().unwrap();
        engine.eval("2 + 2").unwrap();
    }

    #[test]
    fn eval_item_then_expression() {
        let mut engine = Engine::new().unwrap();
        engine.eval("fn double(x: i32) -> i32 { x * 2 }").unwrap();
        engine.eval("double(3)").unwrap();
    }

    #[test]
    fn eval_statement() {
        let mut engine = Engine::new().unwrap();
        engine.eval("let _x = 1").unwrap();
    }

    #[test]
    fn eval_compile_error() {
        let mut engine = Engine::new().unwrap();
        let err = engine.eval("this is not rust").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("error") || format!("{err}").contains("error"),
            "{err}"
        );
    }
}
