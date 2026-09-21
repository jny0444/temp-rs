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
    loaders: Vec<MiniLoader>,
    item_history: Vec<String>,
    binding_history: Vec<String>,
    counter: usize,
}

impl Engine {
    pub fn new() -> Result<Self> {
        let scratch_dir = TempDir::new().context("failed to create REPL scratch directory")?;
        Ok(Self {
            scratch_dir,
            loaders: Vec::new(),
            item_history: Vec::new(),
            binding_history: Vec::new(),
            counter: 0,
        })
    }

    pub fn eval(&mut self, snippet: &str) -> Result<()> {
        let kind = classify_input(snippet);

        let source = generate_source(&kind, &self.item_history, &self.binding_history);

        let artifact = compile_cdylib(&source, self.counter, self.scratch_dir.path())?;
        self.counter += 1;

        let loader = MiniLoader::open(&artifact.dylib_path)?;
        self.loaders.push(loader);
        let loader = self.loaders.last().unwrap();

        unsafe {
            let eval_fn = loader.get_symbol::<ReplEvalFn>("__repl_eval")?;
            eval_fn(std::ptr::null_mut());
        }

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
    fn eval_let_then_use() {
        let mut engine = Engine::new().unwrap();
        engine.eval("let a = 3").unwrap();
        engine.eval(r#"println!("{}", a)"#).unwrap();
        engine.eval("a").unwrap();
    }

    #[test]
    fn eval_mut_let_then_assign() {
        let mut engine = Engine::new().unwrap();
        engine.eval("let mut a = 3").unwrap();
        engine.eval("a = 4").unwrap();
        engine.eval("a").unwrap();
    }

    #[test]
    fn eval_vec_push_then_use() {
        let mut engine = Engine::new().unwrap();
        engine.eval("let mut a = vec![1]").unwrap();
        engine.eval("a.push(2)").unwrap();
        engine.eval("a").unwrap();
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
