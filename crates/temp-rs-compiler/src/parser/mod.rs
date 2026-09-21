use syn::{Item, Stmt};

#[derive(Debug, PartialEq, Eq)]
pub enum InputKind {
    Item(String),
    Statement(String),
    Expression(String),
}

pub fn classify_input(raw: &str) -> InputKind {
    let trimmed = raw.trim();

    if is_item(trimmed) {
        return InputKind::Item(trimmed.to_string());
    }

    if is_statement(trimmed) {
        return InputKind::Statement(trimmed.to_string());
    }

    InputKind::Expression(trimmed.to_string())
}

fn is_item(src: &str) -> bool {
    if let Ok(item) = syn::parse_str::<Item>(src) {
        return !is_stmt_like_macro(&item);
    }

    syn::parse_str::<syn::File>(src)
        .is_ok_and(|file| !file.items.is_empty() && !file.items.iter().all(is_stmt_like_macro))
}

fn is_stmt_like_macro(item: &Item) -> bool {
    match item {
        Item::Macro(mac) => !mac.mac.path.is_ident("macro_rules"),
        _ => false,
    }
}

fn is_statement(src: &str) -> bool {
    if is_local(src) {
        return true;
    }

    src.ends_with(';') && syn::parse_str::<Stmt>(src).is_ok()
}

fn is_local(src: &str) -> bool {
    parses_as_local(src) || parses_as_local(&format!("{src};"))
}

fn parses_as_local(src: &str) -> bool {
    syn::parse_str::<Stmt>(src).is_ok_and(|stmt| matches!(stmt, Stmt::Local(_)))
}

#[cfg(test)]
mod tests {
    use super::{InputKind, classify_input};

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
    }

    #[test]
    fn classifies_statements() {
        assert_eq!(classify_input("let x = 1;"), stmt("let x = 1;"));
        assert_eq!(classify_input("let x = 1"), stmt("let x = 1"));
        assert_eq!(
            classify_input("println!(\"hi\");"),
            stmt("println!(\"hi\");")
        );
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
}
