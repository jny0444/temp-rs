use temp_rs_core::Engine;

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
