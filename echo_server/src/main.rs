#![allow(clippy::unwrap_used, clippy::disallowed_methods)] // We are just testing here.

use std::{net::TcpListener, thread::spawn};

fn main() {
    let bind_addr = "127.0.0.1:9001";
    let server = TcpListener::bind(bind_addr).unwrap();
    eprintln!("Listening on: ws://{bind_addr}");
    for stream in server.incoming() {
        spawn(move || {
            let mut websocket = tungstenite::accept(stream.unwrap()).unwrap();
            eprintln!("New client connected");
            loop {
                let msg = websocket.read().unwrap();

                // We do not want to send back ping/pong messages.
                if msg.is_binary() || msg.is_text() {
                    if let Err(err) = websocket.send(msg) {
                        eprintln!("Error sending message: {err}");
                        break;
                    } else {
                        eprintln!("Responded.");
                    }
                } else {
                    eprintln!("Message recevied not text or binary.");
                }
            }
            eprintln!("Client left.");
        });
    }
}
