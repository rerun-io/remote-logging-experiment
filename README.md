# Rust Observability Experiment

[![dependency status](https://deps.rs/repo/github/emilk/websocket_experiment/status.svg)](https://deps.rs/repo/github/emilk/websocket_experiment)
[![Build Status](https://github.com/emilk/websocket_experiment/workflows/CI/badge.svg)](https://github.com/emilk/websocket_experiment/actions?workflow=CI)

This is an experiemnt in communicating log events (from [`tracing`](https://crates.io/crates/tracing/)) over websockets, from an app to a viewer.

## Testing it

The server handles communication between the "logger" and the "viewer":
`cargo run --release -p pub_sub_server`

The viewer shows the logging data as it comes in:
`cargo run --release -p viewer`

Log some data:
`cargo run --release -p logger`

## Why?
This is an experiment in how to architecture observaility.
