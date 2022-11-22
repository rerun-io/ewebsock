## Unreleased
* Fix: `WsEvent::Closed` is correctly sent on web when socket is closed.
* Fix: On web, close connection when dropping `WsSender`.

## 0.2.0 - 2022-04-08
* Support WSS (WebSocket Secure) / TLS.
* Improve error reporting.
* `EventHandler` no longer needs to be `Sync`.

## 0.1.0 - 2022-02-23
Initial commit: a simple WebSocket client library that works on both native and on the web.
