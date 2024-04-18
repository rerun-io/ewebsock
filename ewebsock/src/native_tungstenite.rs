#![allow(deprecated)] // TODO(emilk): Remove when we update tungstenite

use std::{
    ops::ControlFlow,
    sync::mpsc::{Receiver, TryRecvError},
};

use crate::{EventHandler, Options, Result, WsEvent, WsMessage};

/// This is how you send [`WsMessage`]s to the server.
///
/// When the last clone of this is dropped, the connection is closed.
pub struct WsSender {
    tx: Option<std::sync::mpsc::Sender<WsMessage>>,
}

impl Drop for WsSender {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::warn!("Failed to close web-socket: {err:?}");
        }
    }
}

impl WsSender {
    /// Send a message.
    ///
    /// You have to wait for [`WsEvent::Opened`] before you can start sending messages.
    pub fn send(&mut self, msg: WsMessage) {
        if let Some(tx) = &self.tx {
            tx.send(msg).ok();
        }
    }

    /// Close the connection.
    ///
    /// This is called automatically when the sender is dropped.
    ///
    /// # Errors
    /// This should never fail, except _maybe_ on Web.
    #[allow(clippy::unnecessary_wraps)] // To keep the same signature as the Web version
    pub fn close(&mut self) -> Result<()> {
        if self.tx.is_some() {
            log::debug!("Closing WebSocket");
        }
        self.tx = None;
        Ok(())
    }

    /// Forget about this sender without closing the connection.
    pub fn forget(mut self) {
        #[allow(clippy::mem_forget)] // intentional
        std::mem::forget(self.tx.take());
    }
}

pub(crate) fn ws_receive_impl(url: String, options: Options, on_event: EventHandler) -> Result<()> {
    std::thread::Builder::new()
        .name("ewebsock".to_owned())
        .spawn(move || {
            if let Err(err) = ws_receiver_blocking(&url, options, &on_event) {
                on_event(WsEvent::Error(err));
            } else {
                log::debug!("WebSocket connection closed.");
            }
        })
        .map_err(|err| format!("Failed to spawn thread: {err}"))?;

    Ok(())
}

/// Connect and call the given event handler on each received event.
///
/// Blocking version of [`ws_receive`], only available on native.
///
/// # Errors
/// All errors are returned to the caller, and NOT reported via `on_event`.
pub fn ws_receiver_blocking(url: &str, options: Options, on_event: &EventHandler) -> Result<()> {
    let config = tungstenite::protocol::WebSocketConfig::from(options);
    let max_redirects = 3; // tungstenite default

    let (mut socket, response) =
        match tungstenite::client::connect_with_config(url, Some(config), max_redirects) {
            Ok(result) => result,
            Err(err) => {
                return Err(format!("Connect: {err}"));
            }
        };

    log::debug!("WebSocket HTTP response code: {}", response.status());
    log::trace!(
        "WebSocket response contains the following headers: {:?}",
        response.headers()
    );

    let control = on_event(WsEvent::Opened);
    if control.is_break() {
        log::trace!("Closing connection due to Break");
        return socket
            .close(None)
            .map_err(|err| format!("Failed to close connection: {err}"));
    }

    loop {
        let control = match socket.read_message() {
            Ok(incoming_msg) => match incoming_msg {
                tungstenite::protocol::Message::Text(text) => {
                    on_event(WsEvent::Message(WsMessage::Text(text)))
                }
                tungstenite::protocol::Message::Binary(data) => {
                    on_event(WsEvent::Message(WsMessage::Binary(data)))
                }
                tungstenite::protocol::Message::Ping(data) => {
                    on_event(WsEvent::Message(WsMessage::Ping(data)))
                }
                tungstenite::protocol::Message::Pong(data) => {
                    on_event(WsEvent::Message(WsMessage::Pong(data)))
                }
                tungstenite::protocol::Message::Close(close) => {
                    on_event(WsEvent::Closed);
                    log::debug!("WebSocket close received: {close:?}");
                    return Ok(());
                }
                tungstenite::protocol::Message::Frame(_) => ControlFlow::Continue(()),
            },
            Err(err) => {
                return Err(format!("read: {err}"));
            }
        };

        if control.is_break() {
            log::trace!("Closing connection due to Break");
            return socket
                .close(None)
                .map_err(|err| format!("Failed to close connection: {err}"));
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

pub(crate) fn ws_connect_impl(
    url: String,
    options: Options,
    on_event: EventHandler,
) -> Result<WsSender> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("ewebsock".to_owned())
        .spawn(move || {
            if let Err(err) = ws_connect_blocking(&url, options, &on_event, &rx) {
                on_event(WsEvent::Error(err));
            } else {
                log::debug!("WebSocket connection closed.");
            }
        })
        .map_err(|err| format!("Failed to spawn thread: {err}"))?;

    Ok(WsSender { tx: Some(tx) })
}

/// Connect and call the given event handler on each received event.
///
/// This is a blocking variant of [`ws_connect`], only available on native.
///
/// # Errors
/// All errors are returned to the caller, and NOT reported via `on_event`.
pub fn ws_connect_blocking(
    url: &str,
    options: Options,
    on_event: &EventHandler,
    rx: &Receiver<WsMessage>,
) -> Result<()> {
    let config = tungstenite::protocol::WebSocketConfig::from(options);
    let max_redirects = 3; // tungstenite default
    let (mut socket, response) =
        match tungstenite::client::connect_with_config(url, Some(config), max_redirects) {
            Ok(result) => result,
            Err(err) => {
                return Err(format!("Connect: {err}"));
            }
        };

    log::debug!("WebSocket HTTP response code: {}", response.status());
    log::trace!(
        "WebSocket response contains the following headers: {:?}",
        response.headers()
    );

    let control = on_event(WsEvent::Opened);
    if control.is_break() {
        log::trace!("Closing connection due to Break");
        return socket
            .close(None)
            .map_err(|err| format!("Failed to close connection: {err}"));
    }

    match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => stream.set_nonblocking(true),

        // tungstenite::stream::MaybeTlsStream::NativeTls(stream) => {
        //     stream.get_mut().set_nonblocking(true)
        // }
        #[cfg(feature = "tls")]
        tungstenite::stream::MaybeTlsStream::Rustls(stream) => {
            stream.get_mut().set_nonblocking(true)
        }
        _ => return Err(format!("Unknown tungstenite stream {:?}", socket.get_mut())),
    }
    .map_err(|err| format!("Failed to make WebSocket non-blocking: {err}"))?;

    loop {
        let mut did_work = false;

        match rx.try_recv() {
            Ok(outgoing_message) => {
                did_work = true;
                let outgoing_message = match outgoing_message {
                    WsMessage::Text(text) => tungstenite::protocol::Message::Text(text),
                    WsMessage::Binary(data) => tungstenite::protocol::Message::Binary(data),
                    WsMessage::Ping(data) => tungstenite::protocol::Message::Ping(data),
                    WsMessage::Pong(data) => tungstenite::protocol::Message::Pong(data),
                    WsMessage::Unknown(_) => panic!("You cannot send WsMessage::Unknown"),
                };
                if let Err(err) = socket.write_message(outgoing_message) {
                    socket.close(None).ok();
                    socket.write_pending().ok();
                    return Err(format!("send: {err}"));
                }
            }
            Err(TryRecvError::Disconnected) => {
                log::debug!("WsSender dropped - closing connection.");
                socket.close(None).ok();
                socket.write_pending().ok();
                return Ok(());
            }
            Err(TryRecvError::Empty) => {}
        };

        let control = match socket.read_message() {
            Ok(incoming_msg) => {
                did_work = true;
                match incoming_msg {
                    tungstenite::protocol::Message::Text(text) => {
                        on_event(WsEvent::Message(WsMessage::Text(text)))
                    }
                    tungstenite::protocol::Message::Binary(data) => {
                        on_event(WsEvent::Message(WsMessage::Binary(data)))
                    }
                    tungstenite::protocol::Message::Ping(data) => {
                        on_event(WsEvent::Message(WsMessage::Ping(data)))
                    }
                    tungstenite::protocol::Message::Pong(data) => {
                        on_event(WsEvent::Message(WsMessage::Pong(data)))
                    }
                    tungstenite::protocol::Message::Close(close) => {
                        on_event(WsEvent::Closed);
                        log::debug!("Close received: {close:?}");
                        return Ok(());
                    }
                    tungstenite::protocol::Message::Frame(_) => ControlFlow::Continue(()),
                }
            }
            Err(tungstenite::Error::Io(io_err))
                if io_err.kind() == std::io::ErrorKind::WouldBlock =>
            {
                ControlFlow::Continue(()) // Ignore
            }
            Err(err) => {
                return Err(format!("read: {err}"));
            }
        };

        if control.is_break() {
            log::trace!("Closing connection due to Break");
            return socket
                .close(None)
                .map_err(|err| format!("Failed to close connection: {err}"));
        }

        if !did_work {
            std::thread::sleep(std::time::Duration::from_millis(10)); // TODO(emilk): make configurable
        }
    }
}
