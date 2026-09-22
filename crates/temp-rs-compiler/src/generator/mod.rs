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
