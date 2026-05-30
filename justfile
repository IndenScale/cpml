# CPML development task runner
# https://github.com/casey/just

_default:
    just --list

# Run all quality checks (clippy + test + fmt)
check:
    cargo clippy -- -D warnings
    cargo test
    cargo fmt --check

# Run clippy with warnings as errors
lint:
    cargo clippy -- -D warnings

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Check formatting
fmt-check:
    cargo fmt --check

# Auto-format code
fmt:
    cargo fmt

# Generate and open documentation
doc:
    cargo doc --open

# Run test coverage (requires cargo-tarpaulin)
coverage:
    cargo tarpaulin --out Html --output-dir coverage

# Run dependency audit (requires cargo-deny)
audit:
    cargo deny check

# Run benchmarks (requires cargo-criterion)
bench:
    cargo criterion

# Install development dependencies
setup:
    cargo install cargo-tarpaulin cargo-deny cargo-criterion cargo-release
    cp scripts/pre-commit .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    @echo "Development environment ready."

# Build release binary
release:
    cargo build --release

# Run the full CI pipeline locally
ci: check audit coverage
