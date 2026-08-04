set dotenv-load := false

# Enter the pinned development environment.
default:
    @just --list

# Run all checks required before a commit.
check: fmt-check lint test package-lint privacy-lint architecture-lint guix-lock-lint guix-syntax-lint

# Format Rust sources in place.
fmt:
    cargo fmt --all

# Verify Rust formatting without changing files.
fmt-check:
    cargo fmt --all -- --check

# Reject compiler and Clippy warnings.
lint:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Run the test suite.
test:
    cargo test --workspace --all-targets --all-features --locked

# Verify that the reusable library builds from its publication archive.
package-lint:
    cargo package --package bip39-ceremony-core --locked --allow-dirty

# Check dependency advisories, licenses, and sources.
security:
    cargo audit
    cargo deny check

# Check for likely credentials and private local details.
privacy-lint:
    python3 scripts/privacy-lint.py

# Enforce dependency direction at the presentation and terminal boundaries.
architecture-lint:
    bash scripts/architecture-lint.sh

# Verify Guix crate origins exactly mirror the Cargo lockfile.
guix-lock-lint:
    python3 scripts/guix-lock-lint.py

# Parse Guix files with Guix's registered Scheme readers.
guix-syntax-lint:
    bash scripts/guix-syntax-lint.sh

# Run the complete local gate.
precommit: check security

# Install the repository-owned pre-commit hook.
hooks:
    git config core.hooksPath .githooks

# Show the reviewed normal dependency graph.
deps:
    cargo tree --workspace --locked -e normal

# Build the debug binary.
build:
    cargo build --workspace --locked

# Build the optimized binary for local development only.
release:
    cargo build --workspace --release --locked

# Build and test the dynamically linked Linux feasibility artifact.
release-gnu:
    nix build .#gnu
    python3 scripts/pty-smoke.py result/bin/bip39-ceremony
    file result/bin/bip39-ceremony

# Build and test the static-musl Linux feasibility artifact.
release-musl:
    nix build .#musl
    python3 scripts/pty-smoke.py result/bin/bip39-ceremony
    file result/bin/bip39-ceremony

# Exercise every release feasibility derivation for this host.
release-feasibility:
    nix flake check

# Run the Ember-themed bordered ceremony panel.
run:
    cargo run --package bip39-ceremony-tui --locked

# Run without ANSI color while retaining structural emphasis.
run-plain:
    BIP39_CEREMONY_THEME=plain cargo run --package bip39-ceremony-tui --locked
