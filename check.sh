#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

cargo install --quiet typos-cli

typos
cargo check --quiet --workspace --all-targets
cargo check --quiet --workspace --all-targets --all-features
cargo check --quiet -p example_app --all-features --lib --target wasm32-unknown-unknown
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --quiet --workspace --all-targets --all-features
cargo test --quiet --workspace --doc

./cargo_deny.sh

echo "All checks passed!"
