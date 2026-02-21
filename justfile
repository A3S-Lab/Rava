# Rava — build recipes

# Default: check + test
default: check test

# Build all crates
build:
    cargo build --workspace

# Build release binary
release:
    cargo build --workspace --release

# Run all tests
test:
    cargo test --workspace

# Lint
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Format
fmt:
    cargo fmt --all

# Run the CLI (dev)
run *ARGS:
    cargo run -p rava -- {{ARGS}}

# Clean
clean:
    cargo clean
