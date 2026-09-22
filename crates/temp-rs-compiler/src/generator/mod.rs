use std::ops::Deref;

use crate::parser::InputKind;

pub struct GeneratedSource {
    pub source: String,
    /// 1-based line of the user's snippet in `source`.
    pub snippet_start_line: usize,
}

impl Deref for GeneratedSource {
    type Target = str;

    fn deref(&self) -> &str {
        &self.source
    }
}

pub fn generate_source(
    kind: &InputKind,
    item_history: &[String],
    binding_history: &[String],
) -> GeneratedSource {
    let mut out = String::new();

    out.push_str("#![allow(unused, unused_imports, dead_code)]\n");

    for item in item_history {
        out.push_str(item);
        out.push('\n');
    }

    let snippet_start_line = match kind {
        InputKind::Item(code) => {
            let start = next_line_number(&out);
            out.push_str(code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
            push_eval_fn(&mut out, "");
            start
        }
        InputKind::Statement(code) => {
            out.push_str(
                r#"
#[no_mangle]
pub extern "C" fn __repl_eval(_ctx: *mut std::ffi::c_void) {
"#,
            );
            push_bindings(&mut out, binding_history);
            let start = next_line_number(&out);
            push_terminated_stmt(&mut out, code);
            out.push_str("}\n");
            start
        }
        InputKind::Expression(code) => {
            out.push_str(
                r#"
fn __repl_print<T: std::fmt::Debug>(value: T) {
    if std::any::type_name::<T>() != "()" {
        println!("{value:?}");
    }
}

#[no_mangle]
pub extern "C" fn __repl_eval(_ctx: *mut std::ffi::c_void) {
"#,
            );
            push_bindings(&mut out, binding_history);
            out.push_str("    __repl_print({\n");
            let start = next_line_number(&out);
            out.push_str(code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("    });\n}\n");
            start
        }
    };

    GeneratedSource {
        source: out,
        snippet_start_line,
    }
}

fn next_line_number(src: &str) -> usize {
    src.bytes().filter(|&b| b == b'\n').count() + 1
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
