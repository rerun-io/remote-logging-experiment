#!/bin/bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

./viewer/build_web.sh --fast
cargo check --workspace --all-targets
cargo check -p viewer -p example_app --all-features --lib --target wasm32-unknown-unknown
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc

cargo deny check

echo "All checks passed!"
