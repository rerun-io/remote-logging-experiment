# Rust Observability Experiment

This is an experiment in communicating log events (from [`tracing`](https://crates.io/crates/tracing/)) over websockets, from an app to a viewer.

This is a just a prototype, and not ready for production.

## Architecture

* The `logger` connects to a `pub_sub_server` with using web-sockets, and sends all log events as they come.
* The `viewer` connects to the same `pub_sub_server` (using the same web-socket protocol) and displays the events.
* The `pub_sub_server` forwards, records and replays the log events.

The viewer is either a native app (`cargo run --release viewer`) or a web app (`./viewer/build_web.sh`). The viewer web app can be served usiong `web_server`.

The `logger` is a library, and should work for web apps too (i.e. for apps compiled to WASM that runs in the browser).

There is an `example_app` that uses `tracing` for logging, sending it to a `pub_sub_server` on `126.0.0.1:9002`. `example_app` also by default starts the `pub_sub_server` and the `web_server` so you don't need to run those seperatedly.

### Future work
So much!

* The logger could store log events until it connects to the `pub_sub_server`.
* The logger should retry the connection to the `pub_sub_server`.
* The logger could log to disk, and the viewer read from disk.
* The `pub_sub_server` could persist data on disk.
* It would be nice if the `pub_sub_server` also served the viewer web app so a separate web server wasn't needed.
* The viewer could be improved a lot
  * Retry web-socket connection
  * Better log message visualization
