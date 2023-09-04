use crate::{EventHandler, Result, WsEvent, WsMessage};

/// This is how you send [`WsMessage`]s to the server.
///
/// When the last clone of this is dropped, the connection is closed.
pub struct WsSender {
    tx: tokio::sync::mpsc::Sender<WsMessage>,
}

impl WsSender {
    /// Send a message.
    ///
    /// You have to wait for [`WsEvent::Opened`] before you can start sending messages.
    pub fn send(&mut self, msg: WsMessage) {
        let tx = self.tx.clone();
        tokio::spawn(async move { tx.send(msg).await });
    }
}

async fn ws_connect_async(
    url: String,
    outgoing_messages_stream: impl futures::Stream<Item = WsMessage>,
    on_event: EventHandler,
) {
    use futures::StreamExt as _;

    let (ws_stream, _) = match tokio_tungstenite::connect_async(url).await {
        Ok(result) => result,
        Err(err) => {
            on_event(WsEvent::Error(err.to_string()));
            return;
        }
    };

    tracing::info!("WebSocket handshake has been successfully completed");
    on_event(WsEvent::Opened);

    let (write, read) = ws_stream.split();

    let writer = outgoing_messages_stream
        .map(|ws_message| match ws_message {
            WsMessage::Text(text) => tungstenite::protocol::Message::Text(text),
            WsMessage::Binary(data) => tungstenite::protocol::Message::Binary(data),
            WsMessage::Ping(data) => tungstenite::protocol::Message::Ping(data),
            WsMessage::Pong(data) => tungstenite::protocol::Message::Pong(data),
            WsMessage::Unknown(_) => panic!("You cannot send WsMessage::Unknown"),
        })
        .map(Ok)
        .forward(write);

    let reader = read.for_each(move |event| {
        match event {
            Ok(message) => match message {
                tungstenite::protocol::Message::Text(text) => {
                    on_event(WsEvent::Message(WsMessage::Text(text)));
                }
                tungstenite::protocol::Message::Binary(data) => {
                    on_event(WsEvent::Message(WsMessage::Binary(data)));
                }
                tungstenite::protocol::Message::Ping(data) => {
                    on_event(WsEvent::Message(WsMessage::Ping(data)));
                }
                tungstenite::protocol::Message::Pong(data) => {
                    on_event(WsEvent::Message(WsMessage::Pong(data)));
                }
                tungstenite::protocol::Message::Close(_) => {
                    on_event(WsEvent::Closed);
                }
                tungstenite::protocol::Message::Frame(_) => {}
            },
            Err(err) => {
                on_event(WsEvent::Error(err.to_string()));
            }
        };
        async {}
    });

    futures_util::pin_mut!(reader, writer);
    futures_util::future::select(reader, writer).await;
}

/// Call the given event handler on each new received event.
///
/// This is a more advanced version of [`crate::connect`].
///
/// # Errors
/// * On native: never.
/// * On web: failure to use `WebSocket` API.
pub fn ws_connect(url: String, on_event: EventHandler) -> Result<WsSender> {
    Ok(ws_connect_native(url, on_event))
}

/// Like [`ws_connect`], but cannot fail. Only available on native builds.
pub fn ws_connect_native(url: String, on_event: EventHandler) -> WsSender {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

    let outgoing_messages_stream = async_stream::stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
        tracing::debug!("WsSender dropped - closing connection.");
    };

    tokio::spawn(async move {
        ws_connect_async(url.clone(), outgoing_messages_stream, on_event).await;
        tracing::debug!("WS connection finished.");
    });
    WsSender { tx }
}
