use crate::parser::InputKind;

pub fn generate_source(
    kind: &InputKind,
    item_history: &[String],
    binding_history: &[String],
) -> String {
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
            push_bindings(&mut body, binding_history);
            push_terminated_stmt(&mut body, code);
            push_eval_fn(&mut out, &body);
        }
        InputKind::Expression(code) => {
            let mut body = String::new();
            push_bindings(&mut body, binding_history);
            body.push_str("    let __repl_val = { ");
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

fn push_bindings(body: &mut String, binding_history: &[String]) {
    for stmt in binding_history {
        push_terminated_stmt(body, stmt);
    }
}

fn push_terminated_stmt(body: &mut String, code: &str) {
    body.push_str(code);
    if !code.trim_end().ends_with(';') {
        body.push(';');
    }
    body.push('\n');
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
        let src = generate_source(&InputKind::Item("fn foo() {}".into()), &[], &[]);
        assert!(src.contains("fn foo() {}"));
        assert!(src.contains("fn __repl_eval"));
        assert!(src.contains("#[no_mangle]"));
    }

    #[test]
    fn history_is_prepended() {
        let src = generate_source(
            &InputKind::Expression("foo()".into()),
            &["fn foo() -> i32 { 1 }".into()],
            &[],
        );
        let foo = src.find("fn foo()").unwrap();
        let eval = src.find("fn __repl_eval").unwrap();
        assert!(foo < eval);
        assert!(src.contains("let __repl_val"));
        assert!(src.contains("println!"));
    }

    #[test]
    fn statement_body_is_inside_eval() {
        let src = generate_source(&InputKind::Statement("let x = 1".into()), &[], &[]);
        let eval = src.find("fn __repl_eval").unwrap();
        let let_pos = src.find("let x = 1").unwrap();
        assert!(let_pos > eval);
    }

    #[test]
    fn lets_are_replayed_before_use() {
        let src = generate_source(
            &InputKind::Statement(r#"println!("{}", a)"#.into()),
            &[],
            &["let a = 3".into()],
        );
        let let_pos = src.find("let a = 3").unwrap();
        let print_pos = src.find("println!").unwrap();
        assert!(let_pos < print_pos);
        assert!(!src.contains(";;"));
    }

    #[test]
    fn assignments_are_replayed_after_lets() {
        let src = generate_source(
            &InputKind::Expression("a".into()),
            &[],
            &["let mut a = 3".into(), "a = 4".into()],
        );
        let let_pos = src.find("let mut a = 3").unwrap();
        let assign_pos = src.find("a = 4").unwrap();
        let use_pos = src.find("let __repl_val").unwrap();
        assert!(let_pos < assign_pos);
        assert!(assign_pos < use_pos);
    }

    #[test]
    fn method_calls_are_replayed() {
        let src = generate_source(
            &InputKind::Expression("a".into()),
            &[],
            &["let mut a = vec![1]".into(), "a.push(2)".into()],
        );
        let let_pos = src.find("let mut a = vec![1]").unwrap();
        let push_pos = src.find("a.push(2)").unwrap();
        assert!(let_pos < push_pos);
    }
}
