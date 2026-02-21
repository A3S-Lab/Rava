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

# Run tests for a specific crate (e.g. just test-crate frontend)
test-crate crate:
    cargo test -p rava-{{crate}}

# Run a single test by name (e.g. just test-one frontend parser::tests::parse_hello_world)
test-one crate name:
    cargo test -p rava-{{crate}} --lib -- {{name}}

# Run all .java examples and diff against .expected output
examples: build
    #!/usr/bin/env bash
    set -e
    pass=0; fail=0
    while IFS= read -r java; do
        expected="${java%.java}.expected"
        [ -f "$expected" ] || continue
        name="${java#examples/}"
        actual=$(./target/debug/rava run "$java" 2>/dev/null)
        if diff -q <(echo "$actual") "$expected" > /dev/null 2>&1; then
            echo "  ✓ $name"
            pass=$((pass + 1))
        else
            echo "  ✗ $name"
            diff <(echo "$actual") "$expected" | sed 's/^/    /'
            fail=$((fail + 1))
        fi
    done < <(find examples -name "*.java" | sort)
    echo ""
    echo "$pass passed, $fail failed"
    [ $fail -eq 0 ]

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
