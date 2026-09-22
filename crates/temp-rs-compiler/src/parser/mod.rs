use syn::{BinOp, Expr, Item, Stmt};

#[derive(Debug, PartialEq, Eq)]
pub enum InputKind {
    Item(String),
    Statement(String),
    Expression(String),
}

pub fn classify_input(raw: &str) -> InputKind {
    let trimmed = raw.trim();

    if let Some(item) = item_source(trimmed) {
        return InputKind::Item(item);
    }

    if is_statement(trimmed) {
        return InputKind::Statement(trimmed.to_string());
    }

    InputKind::Expression(trimmed.to_string())
}

/// Item text to replay at crate root.
///
/// A braced item with an extra trailing `;` (`struct A { n: u8 };`) is not a
/// valid crate-level item. Drop that semicolon when the rest parses as an item.
/// Semicolons that belong to the item (`struct Foo;`, `const N: i32 = 1;`) stay.
fn item_source(src: &str) -> Option<String> {
    if is_item(src) {
        return Some(src.to_string());
    }

    let stripped = src.trim_end_matches(';').trim_end();
    if stripped.len() < src.len() && is_item(stripped) {
        Some(stripped.to_string())
    } else {
        None
    }
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
