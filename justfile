# Justfile - Development Commands (ADR-002)

# Default task
default:
    @just --list

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run clippy
lint:
    cargo clippy -- -D warnings

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Build (debug)
build:
    cargo build

# Build (release)
build-release:
    cargo build --release

# Run
run:
    cargo run

# Clean build artifacts
clean:
    cargo clean

# Full check (format + lint + test + build)
check: fmt-check lint test build

# Development loop (format + lint + test)
dev: fmt lint test

# Watch for changes and run tests
watch:
    cargo watch -x test

# Watch for changes and run
watch-run:
    cargo watch -x run

