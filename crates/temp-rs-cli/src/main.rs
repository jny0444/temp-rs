use std::{iter::Peekable, mem::take, str::Chars};

use anyhow::Result;
use rustyline::DefaultEditor;
use temp_rs_core::Engine;

fn main() -> Result<()> {
    let mut engine = Engine::new()?;
    let mut rl = DefaultEditor::new()?;
    let mut pending = String::new();

    loop {
        let prompt = if pending.is_empty() { "-> " } else { ".. " };
        match rl.readline(prompt) {
            Ok(line) => {
                if !pending.is_empty() {
                    pending.push('\n');
                }
                pending.push_str(&line);

                if input_needs_more(&pending) {
                    continue;
                }

                let snippet = take(&mut pending);
                let trimmed = snippet.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(trimmed);

                if let Err(err) = engine.eval(trimmed) {
                    eprintln!("{err}");
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                pending.clear();
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("readline error: {err}");
                break;
            }
        }
    }

    Ok(())
}

fn input_needs_more(src: &str) -> bool {
    let mut chars = src.chars().peekable();
    let mut stack = Vec::new();
    // Set when `#` is immediately followed by `[`, so that `[` is part of the
    // attribute instead of a new token after one.
    let mut saw_hash = false;
    // Depth of `stack` while an outer attribute or outer doc comment is the
    // last real token. Cleared by the next token outside that group.
    let mut outer_attr_depth: Option<usize> = None;

    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                let outer_doc = chars.peek() == Some(&'/');
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
                if outer_doc {
                    outer_attr_depth = Some(stack.len());
                }
                saw_hash = false;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let outer_doc = if chars.peek() == Some(&'*') {
                    let mut look = chars.clone();
                    look.next();
                    look.peek() != Some(&'*')
                } else {
                    false
                };
                if !skip_block_comment(&mut chars) {
                    return true;
                }
                if outer_doc {
                    outer_attr_depth = Some(stack.len());
                }
                saw_hash = false;
            }
            '"' => {
                finish_outer_attr(&mut outer_attr_depth, stack.len());
                saw_hash = false;
                if !skip_cooked_string(&mut chars) {
                    return true;
                }
            }
            'b' | 'c' | 'r' => {
                finish_outer_attr(&mut outer_attr_depth, stack.len());
                saw_hash = false;
                if let Some(lit) = take_prefixed_literal(c, &mut chars) {
                    let closed = match lit {
                        OpenLit::Cooked => skip_cooked_string(&mut chars),
                        OpenLit::Raw(hashes) => skip_raw_string(&mut chars, hashes),
                        OpenLit::Char => consume_char_literal(&mut chars),
                    };
                    if !closed {
                        return true;
                    }
                }
            }
            '\'' => {
                finish_outer_attr(&mut outer_attr_depth, stack.len());
                saw_hash = false;
                if chars.peek().is_some_and(|n| *n == '_' || n.is_alphabetic()) {
                    while chars
                        .peek()
                        .is_some_and(|n| *n == '_' || n.is_alphanumeric())
                    {
                        chars.next();
                    }
                } else if !consume_char_literal(&mut chars) {
                    return true;
                }
            }
            '#' => {
                if chars.peek() == Some(&'[') {
                    saw_hash = true;
                    outer_attr_depth = Some(stack.len());
                } else {
                    saw_hash = false;
                    finish_outer_attr(&mut outer_attr_depth, stack.len());
                }
            }
            '(' | '[' | '{' => {
                let opens_outer_attr = c == '[' && saw_hash;
                saw_hash = false;
                if !opens_outer_attr {
                    finish_outer_attr(&mut outer_attr_depth, stack.len());
                }
                stack.push(c);
            }
            ')' | ']' | '}' => {
                saw_hash = false;
                let expected = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if stack.pop() != Some(expected) {
                    return false;
                }
            }
            _ => {
                saw_hash = false;
                if !c.is_whitespace() {
                    finish_outer_attr(&mut outer_attr_depth, stack.len());
                }
            }
        }
    }

    !stack.is_empty() || outer_attr_depth.is_some()
}

fn finish_outer_attr(depth: &mut Option<usize>, stack_len: usize) {
    if *depth == Some(stack_len) {
        *depth = None;
    }
}

enum OpenLit {
    Cooked,
    Raw(usize),
    Char,
}

fn take_prefixed_literal(first: char, chars: &mut Peekable<Chars<'_>>) -> Option<OpenLit> {
    match first {
        'b' if chars.peek() == Some(&'\'') => {
            chars.next();
            Some(OpenLit::Char)
        }
        'b' | 'c' => match chars.peek().copied() {
            Some('"') => {
                chars.next();
                Some(OpenLit::Cooked)
            }
            Some('r') => {
                chars.next();
                take_raw(chars)
            }
            _ => None,
        },
        'r' => take_raw(chars),
        _ => None,
    }
}

fn take_raw(chars: &mut Peekable<Chars<'_>>) -> Option<OpenLit> {
    let mut look = chars.clone();
    let mut hashes = 0;
    while look.peek() == Some(&'#') {
        look.next();
        hashes += 1;
    }
    if look.peek() != Some(&'"') {
        return None;
    }
    for _ in 0..hashes {
        chars.next();
    }
    chars.next();
    Some(OpenLit::Raw(hashes))
}

fn skip_cooked_string(chars: &mut Peekable<Chars<'_>>) -> bool {
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' => return true,
            _ => {}
        }
    }
    false
}
fn skip_raw_string(chars: &mut Peekable<Chars<'_>>, hashes: usize) -> bool {
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut look = chars.clone();
        if (0..hashes).all(|_| look.next() == Some('#')) {
            for _ in 0..hashes {
                chars.next();
            }
            return true;
        }
    }
    false
}

fn skip_block_comment(chars: &mut Peekable<Chars<'_>>) -> bool {
    let mut depth = 1;
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            depth += 1;
        } else if c == '*' && chars.peek() == Some(&'/') {
            chars.next();
            depth -= 1;
            if depth == 0 {
                return true;
            }
        }
    }
    false
}
fn consume_char_literal(chars: &mut Peekable<Chars<'_>>) -> bool {
    match chars.next() {
        Some('\\') => {
            if !consume_escape(chars) {
                return false;
            }
        }
        Some(_) => {}
        None => return false,
    }
    if chars.peek() == Some(&'\'') {
        chars.next();
        true
    } else {
        false
    }
}
fn consume_escape(chars: &mut Peekable<Chars<'_>>) -> bool {
    match chars.next() {
        Some('u') if chars.peek() == Some(&'{') => {
            chars.next();
            for c in chars.by_ref() {
                if c == '}' {
                    return true;
                }
            }
            false
        }
        Some(_) => true,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::input_needs_more;

    #[test]
    fn waits_after_outer_attribute() {
        assert!(input_needs_more("#[derive(Debug)]"));
        assert!(input_needs_more("#[derive(Debug)]\n#[allow(dead_code)]"));
        assert!(input_needs_more("#[derive(Debug)] // kept"));
        assert!(!input_needs_more("#[derive(Debug)]\nstruct Foo;"));
        assert!(!input_needs_more("#[derive(Debug)] struct Foo;"));
    }

    #[test]
    fn waits_after_outer_doc_comment() {
        assert!(input_needs_more("/// A point"));
        assert!(input_needs_more("/** A point */"));
        assert!(!input_needs_more("/// A point\nstruct Foo;"));
        assert!(!input_needs_more("/* ordinary */"));
    }

    #[test]
    fn still_tracks_delimiters_and_strings() {
        assert!(input_needs_more("struct A {"));
        assert!(!input_needs_more("struct A {\n    x: i32,\n}"));
        assert!(!input_needs_more("println!(\"{name}\")"));
        assert!(input_needs_more("println!(\"{"));
    }
}
