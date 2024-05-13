impl From<crate::Options> for tungstenite::protocol::WebSocketConfig {
    fn from(options: crate::Options) -> Self {
        let crate::Options {
            max_incoming_frame_size,
            ..
        } = options;

        tungstenite::protocol::WebSocketConfig {
            max_frame_size: if max_incoming_frame_size == usize::MAX {
                None
            } else {
                Some(max_incoming_frame_size)
            },
            ..Default::default()
        }
    }
}
