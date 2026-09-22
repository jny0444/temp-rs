use std::collections::HashSet;

use syn::{BinOp, Expr, Item, Path, Stmt, Type, UseTree};

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

/// Identity of a crate-level item, used to replace rather than duplicate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemKey {
    Fn(String),
    Struct(String),
    Enum(String),
    Union(String),
    Trait(String),
    Type(String),
    Const(String),
    Static(String),
    Mod(String),
    Macro(String),
    Use(String),
    Impl {
        self_ty: String,
        trait_: Option<String>,
    },
}

/// Keys for every item in `src` (one snippet may contain several).
pub fn item_keys(src: &str) -> HashSet<ItemKey> {
    let mut keys = HashSet::new();
    if let Ok(file) = syn::parse_str::<syn::File>(src) {
        for item in file.items {
            if let Some(key) = item_key(&item) {
                keys.insert(key);
            }
        }
        return keys;
    }
    if let Ok(item) = syn::parse_str::<Item>(src)
        && let Some(key) = item_key(&item)
    {
        keys.insert(key);
    }
    keys
}

fn item_key(item: &Item) -> Option<ItemKey> {
    Some(match item {
        Item::Fn(item) => ItemKey::Fn(item.sig.ident.to_string()),
        Item::Struct(item) => ItemKey::Struct(item.ident.to_string()),
        Item::Enum(item) => ItemKey::Enum(item.ident.to_string()),
        Item::Union(item) => ItemKey::Union(item.ident.to_string()),
        Item::Trait(item) => ItemKey::Trait(item.ident.to_string()),
        Item::TraitAlias(item) => ItemKey::Trait(item.ident.to_string()),
        Item::Type(item) => ItemKey::Type(item.ident.to_string()),
        Item::Const(item) => ItemKey::Const(item.ident.to_string()),
        Item::Static(item) => ItemKey::Static(item.ident.to_string()),
        Item::Mod(item) => ItemKey::Mod(item.ident.to_string()),
        Item::Macro(item) => ItemKey::Macro(
            item.ident
                .as_ref()
                .map(|id| id.to_string())
                .or_else(|| item.mac.path.get_ident().map(|id| id.to_string()))?,
        ),
        Item::Use(item) => ItemKey::Use(use_key(&item.tree)),
        Item::Impl(item) => ItemKey::Impl {
            self_ty: type_key(&item.self_ty),
            trait_: item.trait_.as_ref().map(|(path, _)| path_key(path)),
        },
        _ => return None,
    })
}

fn path_key(path: &Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn type_key(ty: &Type) -> String {
    match ty {
        Type::Path(ty) => path_key(&ty.path),
        Type::Reference(ty) => {
            let mut_ = if ty.mutability.is_some() { "mut " } else { "" };
            format!("&{mut_}{}", type_key(&ty.elem))
        }
        Type::Slice(ty) => format!("[{}]", type_key(&ty.elem)),
        Type::Array(ty) => format!("[{}]", type_key(&ty.elem)),
        Type::Ptr(ty) => format!("*{}", type_key(&ty.elem)),
        Type::Paren(ty) => type_key(&ty.elem),
        Type::Tuple(ty) => {
            let inner = ty.elems.iter().map(type_key).collect::<Vec<_>>().join(",");
            format!("({inner})")
        }
        Type::Never(_) => "!".into(),
        Type::Infer(_) => "_".into(),
        _ => "_".into(),
    }
}

fn use_key(tree: &UseTree) -> String {
    match tree {
        UseTree::Path(path) => format!("{}::{}", path.ident, use_key(&path.tree)),
        UseTree::Name(name) => name.ident.to_string(),
        UseTree::Rename(rename) => format!("{} as {}", rename.ident, rename.rename),
        UseTree::Glob(_) => "*".into(),
        UseTree::Group(group) => {
            let mut parts: Vec<_> = group.items.iter().map(use_key).collect();
            parts.sort();
            parts.join(",")
        }
    }
}
