//! A [`WebSocket`](https://en.wikipedia.org/wiki/WebSocket) client library that can be compiled to both native and the web (WASM).
//!
//! Usage:
//! ``` no_run
//! let (mut sender, receiver) = ewebsock::connect("ws://example.com").unwrap();
//! sender.send(ewebsock::WsMessage::Text("Hello!".into()));
//! while let Some(event) = receiver.try_recv() {
//!     println!("Received {:?}", event);
//! }
//! ```

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "websocket")]
pub mod native_websocket;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "with_tungstenite")]
pub mod native_tungstenite;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "with_tungstenite")]
pub use native_tungstenite::*;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "websocket")]
#[cfg(not(feature = "with_tungstenite"))]
pub use native_websocket::*;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
pub use web::*;

// ----------------------------------------------------------------------------

/// A web-socket message.
#[derive(Clone, Debug)]
pub enum WsMessage {
    /// Binary message.
    Binary(Vec<u8>),

    /// Text message.
    Text(String),

    /// Incoming message of unknown type.
    /// You cannot send these.
    Unknown(String),

    /// Only for native.
    Ping(Vec<u8>),

    /// Only for native.
    Pong(Vec<u8>),
}

/// Something happening with the connection.
#[derive(Clone, Debug)]
pub enum WsEvent {
    Opened,
    Message(WsMessage),
    Error(String),
    Closed,
}

pub struct WsReceiver {
    rx: std::sync::mpsc::Receiver<WsEvent>,
}

impl WsReceiver {
    /// Returns a receiver and an event-handler that can be passed to `crate::ws_connect`.
    pub fn new() -> (Self, EventHandler) {
        Self::new_with_callback(|| {})
    }

    /// The given callback will be called on each new message.
    ///
    /// This can be used to wake up the UI thread.
    pub fn new_with_callback(wake_up: impl Fn() + Send + Sync + 'static) -> (Self, EventHandler) {
        let (tx, rx) = std::sync::mpsc::channel();
        let on_event = Box::new(move |event| {
            wake_up(); // wake up UI thread
            if tx.send(event).is_ok() {
                std::ops::ControlFlow::Continue(())
            } else {
                std::ops::ControlFlow::Break(())
            }
        });
        let ws_receiver = WsReceiver { rx };
        (ws_receiver, on_event)
    }

    pub fn try_recv(&self) -> Option<WsEvent> {
        self.rx.try_recv().ok()
    }
}

pub type Error = String;
pub type Result<T> = std::result::Result<T, Error>;

pub type EventHandler = Box<dyn Send + Fn(WsEvent) -> std::ops::ControlFlow<()>>;

/// The easiest to use function.
///
/// # Errors
/// * On native: never.
/// * On web: failure to use `WebSocket` API.
pub fn connect(url: impl Into<String>) -> Result<(WsSender, WsReceiver)> {
    let (ws_receiver, on_event) = WsReceiver::new();
    let ws_sender = ws_connect(url.into(), on_event)?;
    Ok((ws_sender, ws_receiver))
}

/// Like [`connect`], but will call the given wake-up function on each incoming event.
///
/// This allows you to wake up the UI thread, for instance.
///
/// # Errors
/// * On native: never.
/// * On web: failure to use `WebSocket` API.
pub fn connect_with_wakeup(
    url: impl Into<String>,
    wake_up: impl Fn() + Send + Sync + 'static,
) -> Result<(WsSender, WsReceiver)> {
    let (receiver, on_event) = WsReceiver::new_with_callback(wake_up);
    let sender = ws_connect(url.into(), on_event)?;
    Ok((sender, receiver))
}
