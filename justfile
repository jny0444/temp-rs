check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace

run:
    cargo run -p temp-rs-cli

install:
    cargo install --path crates/temp-rs-cli