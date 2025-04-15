## [Unreleased](https://github.com/rerun-io/ewebsock/compare/latest...HEAD)
* CloseEvent error code provided [#59](https://github.com/rerun-io/ewebsock/issues/59) by [@dzmitry-lahoda](https://github.com/dzmitry-lahoda)


## 0.8.0 - 2024-11-11 - Fix native performance bug
* Fix blocking receiver sleeping after every read [#48](https://github.com/rerun-io/ewebsock/pull/48) by [@jprochazk](https://github.com/jprochazk)


## [0.7.0](https://github.com/rerun-io/ewebsock/compare/0.6.0...0.7.0) - 2024-10-10
* Fix crash when error_event does not have "message" or "error" fields [#37](https://github.com/rerun-io/ewebsock/pull/37) (thanks [@romamik](https://github.com/romamik)!)
* Add `Options::additional_headers` and `subprotocols` [#27](https://github.com/rerun-io/ewebsock/pull/27) (thanks [@Its-Just-Nans](https://github.com/Its-Just-Nans)!)
* Update to `tungstenite` 0.23 [#39](https://github.com/rerun-io/ewebsock/pull/39) (thanks [@Its-Just-Nans](https://github.com/Its-Just-Nans)!)
* Add support for tungstenite 0.24 [#46](https://github.com/rerun-io/ewebsock/pull/46)


## [0.6.0](https://github.com/rerun-io/ewebsock/compare/0.5.0...0.6.0) - 2024-05-21
* Allow closing the connecting by returning `ControlFlow::Break` [#33](https://github.com/rerun-io/ewebsock/pull/33)
* Update MSRV to Rust 1.76 [#35](https://github.com/rerun-io/ewebsock/pull/35)


## [0.5.0](https://github.com/rerun-io/ewebsock/compare/0.4.1...0.5.0) - 2024-02-26
* Add `Options` for controlling max frame size of incoming messages - ([#29](https://github.com/rerun-io/ewebsock/pull/29))


## [0.4.1](https://github.com/rerun-io/ewebsock/compare/0.4.0...0.4.1) - 2024-02-15
* Fix: all errors are reported to the caller via `on_event` ([#26](https://github.com/rerun-io/ewebsock/pull/26))
* Add support for tungstenite 0.21, update MSRV to 1.72 ([#28](https://github.com/rerun-io/ewebsock/pull/28))


## [0.4.0](https://github.com/rerun-io/ewebsock/compare/0.3.0...0.4.0) - 2023-10-07
* Make `tokio` an opt-in dependency, and add a simpler `ws_receive` function ([#24](https://github.com/rerun-io/ewebsock/pull/24))


## [0.3.0](https://github.com/rerun-io/ewebsock/compare/0.2.0...0.3.0) - 2023-09-29
* Fix: `WsEvent::Closed` is correctly sent on web when socket is closed (#6)
* Fix: On web, close connection when dropping `WsSender` (#8)


## 0.2.0 - 2022-04-08
* Support WSS (WebSocket Secure) / TLS.
* Improve error reporting.
* `EventHandler` no longer needs to be `Sync`.


## 0.1.0 - 2022-02-23
Initial commit: a simple WebSocket client library that works on both native and on the web.
