# Build all Rust workspace crates.
build:
    cargo build --workspace

# Run all tests.
test:
    cargo test --workspace

# Run the CLI, forwarding all arguments.
run *args:
    cargo run {{args}}
