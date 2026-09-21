use syn::{Item, Stmt};

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
    if syn::parse_str::<Item>(src).is_ok() {
        return true;
    }

    syn::parse_str::<syn::File>(src).is_ok_and(|file| !file.items.is_empty())
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
