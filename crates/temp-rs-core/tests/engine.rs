use temp_rs_core::Engine;

#[test]
fn eval_expression() {
    let mut engine = Engine::new().unwrap();
    engine.eval("2 + 2").unwrap();
}

#[test]
fn eval_braced_struct_with_trailing_semicolon_then_construct() {
    let mut engine = Engine::new().unwrap();
    engine
        .eval("#[derive(Debug)]\nstruct A {\n    n: u8,\n};")
        .unwrap();
    engine.eval("let a = A { n: 10 };").unwrap();
    engine.eval("a").unwrap();
}

#[test]
fn eval_item_then_expression() {
    let mut engine = Engine::new().unwrap();
    engine.eval("fn double(x: i32) -> i32 { x * 2 }").unwrap();
    engine.eval("double(3)").unwrap();
}

#[test]
fn eval_redefines_fn() {
    let mut engine = Engine::new().unwrap();
    engine.eval("fn answer() -> i32 { 1 }").unwrap();
    engine.eval("fn answer() -> i32 { 2 }").unwrap();
    engine.eval("answer()").unwrap();
}

#[test]
fn eval_redefines_struct() {
    let mut engine = Engine::new().unwrap();
    engine
        .eval("#[derive(Debug)] struct Point { x: i32 }")
        .unwrap();
    engine
        .eval("#[derive(Debug)] struct Point { x: i32, y: i32 }")
        .unwrap();
    engine.eval("Point { x: 1, y: 2 }").unwrap();
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

#[test]
fn eval_compile_error_points_at_snippet() {
    let mut engine = Engine::new().unwrap();
    let err = engine.eval(r#"1 + "a""#).unwrap_err();
    let msg = format!("{err}");
    assert!(!msg.contains("eval_"), "{msg}");
    assert!(msg.contains("<repl>:1:"), "{msg}");
}

#[test]
fn eval_unit_expression_succeeds() {
    let mut engine = Engine::new().unwrap();
    engine.eval("()").unwrap();
}
