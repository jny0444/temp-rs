# temp-rs

A lightweight, experimental Rust REPL (Read-Eval-Print Loop) that allows you to interactively run Rust code snippets.

## Features

- **Evaluate Expressions**: Instantly see the results of Rust expressions.
- **Define Items**: Define functions, structs, enums, and macros that persist across sessions.
- **Maintain State**: `let` bindings, assignments, and method calls (like `vec.push()`) are preserved, allowing you to build state incrementally.
- **On-the-fly Compilation**: Compiles your snippets into dynamic libraries and loads them into the running process.

## How it Works

`temp-rs` uses a unique "replay" architecture to simulate a persistent environment:

1.  **Parsing**: Each line of input is classified as an **Item** (e.g., `fn`), a **Statement** (e.g., `let x = 1;`), or an **Expression** (e.g., `x + 1`).
2.  **Source Generation**: For every new input, the engine generates a temporary Rust source file. This file includes:
    - All previously defined **Items**.
    - All previous **Statements** that modify state (like `let` bindings or `mut` updates).
    - The new input wrapped in a special `extern "C"` function named `__repl_eval`.
3.  **Compilation**: The engine calls `rustc` to compile the generated source into a dynamic library (`cdylib`).
4.  **Loading**: The resulting library is loaded using `dlopen` (or equivalent).
5.  **Execution**: The engine lookups the `__repl_eval` symbol and executes it, printing the result if the input was an expression.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable version recommended)

### Running the CLI

You can start the REPL by running the CLI crate:

```bash
cargo run -p temp-rs-cli
```

### Usage Examples

Once the REPL is running, you can type Rust code at the `->` prompt:

**Expressions:**
```rust
-> 2 + 2
4
```

**Variables and State:**
```rust
-> let mut x = 10
-> x += 5
-> x
15
```

**Functions and Items:**
```rust
-> fn double(n: i32) -> i32 { n * 2 }
-> double(x)
30
```

**Complex Types:**
```rust
-> let mut v = vec![1, 2]
-> v.push(3)
-> v
[1, 2, 3]
```

## Project Structure

- `crates/temp-rs-cli`: The command-line interface and REPL loop.
- `crates/temp-rs-core`: The high-level engine that manages history and evaluation.
- `crates/temp-rs-compiler`: Handles Rust code parsing, source generation, and `rustc` invocation.
- `crates/temp-rs-dylib-loader`: A minimal wrapper for loading and searching dynamic libraries.

## Limitations

- Because every evaluation recompiles the entire history of items and bindings, performance may degrade as the session history grows very large.
- External crate dependencies are not yet supported in the REPL environment.
