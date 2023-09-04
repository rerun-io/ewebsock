use crate::{EventHandler, Result, WsEvent, WsMessage};

/// This is how you send messages to the server.
///
/// When the last clone of this is dropped, the connection is closed.
pub struct WsSender {
    sender: websocket::sender::Writer<websocket::sync::stream::TcpStream>,
}

impl WsSender {
    pub fn send(&mut self, msg: WsMessage) -> Result<()> {
        let result = match msg {
            WsMessage::Binary(data) => self
                .sender
                .send_message(&websocket::OwnedMessage::Binary(data)),
            WsMessage::Text(text) => self
                .sender
                .send_message(&websocket::OwnedMessage::Text(text)),
            unknown => {
                panic!("Don't know how to send message: {:?}", unknown);
            }
        };
        result.map_err(|err| err.to_string())
    }
}

/// Call the given event handler on each new received event.
///
/// This is a more advanced version of [`crate::connect`].
///
/// # Errors
/// * On native: never.
/// * On web: failure to use `WebSocket` API.
pub fn ws_connect(url: String, on_event: EventHandler) -> Result<WsSender> {
    let client = websocket::ClientBuilder::new(&url)
        .map_err(|err| err.to_string())?
        .connect_insecure()
        .map_err(|err| err.to_string())?;

    let (mut reader, sender) = client.split().map_err(|err| err.to_string())?;

    std::thread::Builder::new()
        .name("websocket_receiver".into())
        .spawn(move || {
            loop {
                match reader.recv_message() {
                    Ok(message) => {
                        let msg = match message {
                            websocket::OwnedMessage::Binary(binary) => WsMessage::Binary(binary),
                            websocket::OwnedMessage::Text(text) => WsMessage::Text(text),
                            websocket::OwnedMessage::Close(close_data) => {
                                eprintln!("Websocket closed: {:#?}", close_data);
                                on_event(WsEvent::Closed);
                                break;
                            }
                            websocket::OwnedMessage::Ping(data) => WsMessage::Ping(data),
                            websocket::OwnedMessage::Pong(data) => WsMessage::Pong(data),
                        };
                        if matches!(
                            on_event(WsEvent::Message(msg)),
                            std::ops::ControlFlow::Break(())
                        ) {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Websocket error: {:#?}", err);
                    }
                }
            }
            eprintln!("Stopping websocket receiver thread")
        })
        .exepct("Failed to spawn thread");

    Ok(WsSender { sender })
}
