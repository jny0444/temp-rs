use temp_rs_compiler::parser::{InputKind, classify_input};

fn item(src: &str) -> InputKind {
    InputKind::Item(src.trim().to_string())
}

fn stmt(src: &str) -> InputKind {
    InputKind::Statement(src.trim().to_string())
}

fn expr(src: &str) -> InputKind {
    InputKind::Expression(src.trim().to_string())
}

#[test]
fn classifies_items() {
    assert_eq!(classify_input("fn foo() {}"), item("fn foo() {}"));
    assert_eq!(classify_input("struct Foo;"), item("struct Foo;"));
    assert_eq!(
        classify_input("const N: i32 = 1;"),
        item("const N: i32 = 1;")
    );
    assert_eq!(
        classify_input("async fn foo() {}"),
        item("async fn foo() {}")
    );
    assert_eq!(
        classify_input("#[derive(Debug)] struct Foo;"),
        item("#[derive(Debug)] struct Foo;")
    );
    assert_eq!(
        classify_input("struct Foo; impl Foo {}"),
        item("struct Foo; impl Foo {}")
    );
    assert_eq!(
        classify_input("macro_rules! m { () => {} }"),
        item("macro_rules! m { () => {} }")
    );
    assert_eq!(
        classify_input("struct A {\n    n: u8,\n};"),
        item("struct A {\n    n: u8,\n}")
    );
    assert_eq!(
        classify_input("#[derive(Debug)]\nstruct A {\n    n: u8,\n};"),
        item("#[derive(Debug)]\nstruct A {\n    n: u8,\n}")
    );
    assert_eq!(classify_input("fn foo() {};"), item("fn foo() {}"));
}

#[test]
fn classifies_statements() {
    assert_eq!(classify_input("let x = 1;"), stmt("let x = 1;"));
    assert_eq!(classify_input("let x = 1"), stmt("let x = 1"));
    assert_eq!(
        classify_input("println!(\"hi\");"),
        stmt("println!(\"hi\");")
    );
    assert_eq!(classify_input("a = 4"), stmt("a = 4"));
    assert_eq!(classify_input("a = 4;"), stmt("a = 4;"));
    assert_eq!(classify_input("a += 1"), stmt("a += 1"));
    assert_eq!(classify_input("let mut a = 3"), stmt("let mut a = 3"));
    assert_eq!(classify_input("a.push(1);"), stmt("a.push(1);"));
}

#[test]
fn classifies_expressions() {
    assert_eq!(classify_input("2 + 2"), expr("2 + 2"));
    assert_eq!(classify_input("foo()"), expr("foo()"));
    assert_eq!(
        classify_input("if true { 1 } else { 2 }"),
        expr("if true { 1 } else { 2 }")
    );
}
