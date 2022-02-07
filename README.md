# websocket experiemnt crate

[![dependency status](https://deps.rs/repo/github/emilk/websocket_experiment/status.svg)](https://deps.rs/repo/github/emilk/websocket_experiment)
[![Build Status](https://github.com/emilk/websocket_experiment/workflows/CI/badge.svg)](https://github.com/emilk/websocket_experiment/actions?workflow=CI)

This is an experiemnt in communicating log events (from `tracing`) over websockets, from an app to a viewer.

## Testing it

The server handles communication between the "logger" and the "viewer":
`cargo run --release -p ws_server`

The viewer shows the logging data as it comes in:
`cargo run --release -p ws_client`

Log some data:
`cargo run --release -p logger`
