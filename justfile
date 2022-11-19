# Use bash on MacOS and Linux
set shell := ["bash", "-cu"]
# Use PowerShell on Windows
set windows-shell := ["pwsh", "-NoLogo", "-Command"]

# Load `.env` file if present
set dotenv-load := true

# Uncomment the following line to view verbose backtraces:
#export RUST_BACKTRACE := "1"

# Deny on warnings found within documentation.
# export RUSTDOCFLAGS := "-D warnings"

# Default to show all available commands if no arguments passed
_default:
    @just --list

# Create an optimized 'release' build
@build:
    cargo build --release --verbose

# Sanity check to ensure the project compiles
@check:
    cargo +nightly fmt --all -- --check
    cargo test --locked
    cargo +nightly clippy --workspace --all-targets -- -D warnings

# Quickly format and run linter
@lint:
    cargo +nightly clippy --workspace --all-targets

# Run performance benchmarks
@bench:
    cargo bench --verbose

# Create an HTML chart showing compilation timings
@timings:
    cargo clean
    cargo build -Z timings

# Run code-quality and CI-related tasks locally
@pre-commit:
#    cargo doc --no-deps --document-private-items --all-features --workspace --verbose

## Testing
# Run unit tests
@test:
    cargo test --workspace -- --quiet

# Run all unit tests (in release mode)
@test-release:
    cargo test --workspace --release --verbose

# Run tests single-threaded for concurrency-related debugging
@test-debug:
    cargo test --locked -- --test-threads=1 --nocapture

## Project management
# Build the crate documentation, failing on any errors
@generate-docs:
    cargo doc --no-deps --document-private-items --all-features --workspace --verbose --open

# Cleans the project sources, then rebuilds them
@refresh-sources:
    cargo clean --verbose
    cargo build --verbose

# Show the versions of required build tools
@versions:
    rustc --version
    cargo --version

## Running
# Run the default binary
@run args:
    cargo run -- {{ args }}

# Run the binary target with the given name
@run-target name:
    cargo run --bin {{ name }}
