use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::client::Request;
use tungstenite::protocol::WebSocketConfig;

pub fn tungstenite_options(url: &str, options: crate::Options) -> (WebSocketConfig, Request) {
    let mut request = url.into_client_request().unwrap();
    if !options.protocols.is_empty() {
        let protocols = options.protocols.join(", ").try_into().unwrap();
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", protocols);
    }

    let max_frame_size =
        (options.max_incoming_frame_size != usize::MAX).then_some(options.max_incoming_frame_size);

    (
        WebSocketConfig {
            max_frame_size,
            ..Default::default()
        },
        request,
    )
}
