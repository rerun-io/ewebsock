#![allow(clippy::unwrap_used, clippy::disallowed_methods)] // We are just testing here.

use std::net::TcpListener;
use std::net::TcpStream;
use std::time::Instant;

use tungstenite::Message;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [kind, action, address] = &args[..] else {
        eprintln!("Usage: stress_test <tcp|ws> <send|recv> <address>");
        return;
    };
    match (kind.as_str(), action.as_str()) {
        ("tcp", "send") => send_tcp(address),
        ("tcp", "recv") => recv_tcp(address),
        ("ws", "send") => send_ws(address),
        ("ws", "recv") => recv_ws(address),
        _ => {
            eprintln!("Usage: stress_test <tcp|ws> <send|recv> <address>");
        }
    }
}

fn send_tcp(address: &str) {
    use std::io::Write as _;

    println!("Connecting to: {address}");
    let mut stream = TcpStream::connect(address).unwrap();

    println!("Sending 1M messages");
    let start = Instant::now();
    for i in 0..1_000_000 {
        stream.write_all(&vec![i as u8; 4 * 1024]).unwrap();
    }
    let duration = start.elapsed();
    println!("Sent all messages in {}ms", duration.as_millis());
}

fn recv_tcp(address: &str) {
    use std::io::Read as _;

    let server = TcpListener::bind(address).unwrap();
    println!("Listening on: {address}");
    println!("Waiting for client");
    let (mut client, _) = server.accept().unwrap();
    let mut buf = vec![0; 4 * 1024];
    println!("Client connected");
    let start = Instant::now();
    for i in 0..1_000_000 {
        client.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], i as u8, "Invalid message");
    }
    let duration = start.elapsed();
    println!("Received all messages in {}ms", duration.as_millis());
}

fn send_ws(address: &str) {
    println!("Connecting to: ws://{address}");
    let stream = TcpStream::connect(address).unwrap();
    let (mut stream, _) = tungstenite::client(format!("ws://{address}"), stream).unwrap();
    println!("{:?}", stream.get_config());

    println!("Sending 1M messages");
    let start = Instant::now();
    for i in 0..1_000_000 {
        stream
            .send(Message::Binary(vec![i as u8; 4 * 1024]))
            .unwrap();
    }
    let duration = start.elapsed();
    println!("Sent all messages in {}ms", duration.as_millis());
}

fn recv_ws(address: &str) {
    let server = TcpListener::bind(address).unwrap();
    println!("Listening on: ws://{address}");
    println!("Waiting for client");
    let (client, _) = server.accept().unwrap();
    let mut client = tungstenite::accept(client).unwrap();
    println!("{:?}", client.get_config());
    println!("Client connected");
    let start = Instant::now();
    for _ in 0..1_000_000 {
        let message = client.read().unwrap();
        assert!(
            matches!(message, Message::Binary(_)),
            "unexpected message: {message:?}"
        );
    }
    let duration = start.elapsed();
    println!("Received all messages in {}ms", duration.as_millis());
}
