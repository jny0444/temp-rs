use crate::parser::InputKind;

pub fn generate_source(kind: &InputKind, item_history: &[String]) -> String {
    let mut out = String::new();

    out.push_str("#![allow(unused, unused_imports, dead_code)]\n");

    for item in item_history {
        out.push_str(item);
        out.push('\n');
    }

    match kind {
        InputKind::Item(code) => {
            out.push_str(code);
            push_eval_fn(&mut out, "");
        }
        InputKind::Statement(code) => {
            let mut body = String::new();
            body.push_str(code);
            body.push(';');
            push_eval_fn(&mut out, &body);
        }
        InputKind::Expression(code) => {
            let mut body = String::from(r#"    let __repl_val = { "#);
            body.push_str(code);
            body.push_str(
                r#" };
    println!("{__repl_val:?}");
"#,
            );
            push_eval_fn(&mut out, &body);
        }
    }

    out
}

fn push_eval_fn(out: &mut String, body: &str) {
    out.push_str(
        r#"
#[no_mangle]
pub extern "C" fn __repl_eval(_ctx: *mut std::ffi::c_void) {
"#,
    );
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use super::generate_source;
    use crate::parser::InputKind;

    #[test]
    fn item_emits_empty_eval() {
        let src = generate_source(&InputKind::Item("fn foo() {}".into()), &[]);
        assert!(src.contains("fn foo() {}"));
        assert!(src.contains("fn __repl_eval"));
        assert!(src.contains("#[no_mangle]"));
    }

    #[test]
    fn history_is_prepended() {
        let src = generate_source(
            &InputKind::Expression("foo()".into()),
            &["fn foo() -> i32 { 1 }".into()],
        );
        let foo = src.find("fn foo()").unwrap();
        let eval = src.find("fn __repl_eval").unwrap();
        assert!(foo < eval);
        assert!(src.contains("let __repl_val"));
        assert!(src.contains("println!"));
    }

    #[test]
    fn statement_body_is_inside_eval() {
        let src = generate_source(&InputKind::Statement("let x = 1".into()), &[]);
        let eval = src.find("fn __repl_eval").unwrap();
        let let_pos = src.find("let x = 1").unwrap();
        assert!(let_pos > eval);
    }
}
