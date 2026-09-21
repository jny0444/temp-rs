use syn::{BinOp, Expr, Item, Stmt};

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
    if is_persistent_binding(src) {
        return true;
    }

    src.ends_with(';') && syn::parse_str::<Stmt>(src).is_ok()
}

/// `let` / `let mut`, assignment (`a = 1`, `a += 1`), or a method call (`a.push(1)`).
/// These are replayed on later evals so collections can grow.
pub fn is_persistent_binding(src: &str) -> bool {
    is_let_binding(src) || is_assignment(src) || is_method_call(src)
}

/// True for `let` / `let mut` bindings (semicolon optional).
pub fn is_let_binding(src: &str) -> bool {
    parses_as_stmt(src).is_some_and(|stmt| matches!(stmt, Stmt::Local(_)))
}

fn is_assignment(src: &str) -> bool {
    match parses_as_stmt(src) {
        Some(Stmt::Expr(expr, _)) => is_assign_expr(&expr),
        _ => false,
    }
}

fn is_method_call(src: &str) -> bool {
    matches!(
        parses_as_stmt(src),
        Some(Stmt::Expr(Expr::MethodCall(_), _))
    )
}

fn is_assign_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Assign(_) => true,
        Expr::Binary(bin) => matches!(
            bin.op,
            BinOp::AddAssign(_)
                | BinOp::SubAssign(_)
                | BinOp::MulAssign(_)
                | BinOp::DivAssign(_)
                | BinOp::RemAssign(_)
                | BinOp::BitXorAssign(_)
                | BinOp::BitAndAssign(_)
                | BinOp::BitOrAssign(_)
                | BinOp::ShlAssign(_)
                | BinOp::ShrAssign(_)
        ),
        _ => false,
    }
}

fn parses_as_stmt(src: &str) -> Option<Stmt> {
    let src = src.trim();
    syn::parse_str(src)
        .ok()
        .or_else(|| syn::parse_str(&format!("{src};")).ok())
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
}
