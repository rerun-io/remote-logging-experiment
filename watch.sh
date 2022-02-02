#!/bin/bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

watchexec -c 'cargo check -p ws_client --lib --target wasm32-unknown-unknown && cargo check'
