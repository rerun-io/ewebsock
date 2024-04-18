#![allow(trivial_casts)]

use crate::{EventHandler, Options, Result, WsEvent, WsMessage};

#[allow(clippy::needless_pass_by_value)]
fn string_from_js_value(s: wasm_bindgen::JsValue) -> String {
    s.as_string().unwrap_or(format!("{s:#?}"))
}

#[allow(clippy::needless_pass_by_value)]
fn string_from_js_string(s: js_sys::JsString) -> String {
    s.as_string().unwrap_or(format!("{s:#?}"))
}

/// This is how you send messages to the server.
///
/// When this is dropped, the connection is closed.
pub struct WsSender {
    ws: Option<web_sys::WebSocket>,
}

impl Drop for WsSender {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::warn!("Failed to close WebSocket: {err:?}");
        }
    }
}

impl WsSender {
    /// Send the message to the server.
    pub fn send(&mut self, msg: WsMessage) {
        if let Some(ws) = &mut self.ws {
            let result = match msg {
                WsMessage::Binary(data) => {
                    ws.set_binary_type(web_sys::BinaryType::Blob);
                    ws.send_with_u8_array(&data)
                }
                WsMessage::Text(text) => ws.send_with_str(&text),
                unknown => {
                    panic!("Don't know how to send message: {unknown:?}");
                }
            };
            if let Err(err) = result.map_err(string_from_js_value) {
                log::error!("Failed to send: {err:?}");
            }
        }
    }

    /// Close the connection.
    ///
    /// This is called automatically when the sender is dropped.
    ///
    /// # Errors
    /// This should never fail, except _maybe_ on Web.
    pub fn close(&mut self) -> Result<()> {
        if let Some(ws) = self.ws.take() {
            log::debug!("Closing WebSocket");
            ws.close().map_err(string_from_js_value)
        } else {
            Ok(())
        }
    }

    /// Forget about this sender without closing the connection.
    pub fn forget(mut self) {
        self.ws = None;
    }
}

pub(crate) fn ws_receive_impl(url: String, options: Options, on_event: EventHandler) -> Result<()> {
    ws_connect_impl(url, options, on_event).map(|sender| sender.forget())
}

#[allow(clippy::needless_pass_by_value)] // For consistency with the native version
pub(crate) fn ws_connect_impl(
    url: String,
    _ignored_options: Options,
    on_event: EventHandler,
) -> Result<WsSender> {
    // Based on https://rustwasm.github.io/wasm-bindgen/examples/websockets.html

    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast as _;

    // Connect to an server
    let ws = web_sys::WebSocket::new(&url).map_err(string_from_js_value)?;

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // Allow it to be shared by the different callbacks:
    let on_event: std::rc::Rc<dyn Send + Fn(WsEvent) -> std::ops::ControlFlow<()>> =
        on_event.into();

    // onmessage callback
    {
        let on_event = on_event.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                on_event(WsEvent::Message(WsMessage::Binary(array.to_vec())));
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                let file_reader = web_sys::FileReader::new().expect("Failed to create FileReader");
                let file_reader_clone = file_reader.clone();
                // create onLoadEnd callback
                let on_event = on_event.clone();
                let onloadend_cb =
                    Closure::wrap(Box::new(
                        move |_e: web_sys::ProgressEvent| match file_reader_clone.result() {
                            Ok(file_reader) => {
                                let array = js_sys::Uint8Array::new(&file_reader);
                                on_event(WsEvent::Message(WsMessage::Binary(array.to_vec())));
                            }
                            Err(err) => {
                                on_event(WsEvent::Error(format!(
                                    "Failed to read binary blob: {}",
                                    string_from_js_value(err)
                                )));
                            }
                        },
                    )
                        as Box<dyn FnMut(web_sys::ProgressEvent)>);
                file_reader.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                file_reader
                    .read_as_array_buffer(&blob)
                    .expect("blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                on_event(WsEvent::Message(WsMessage::Text(string_from_js_string(
                    txt,
                ))));
            } else {
                log::debug!("Unknown websocket message received: {:?}", e.data());
                on_event(WsEvent::Message(WsMessage::Unknown(string_from_js_value(
                    e.data(),
                ))));
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        // forget the callback to keep it alive
        onmessage_callback.forget();
    }

    {
        let on_event = on_event.clone();
        let onerror_callback = Closure::wrap(Box::new(move |error_event: web_sys::ErrorEvent| {
            log::error!(
                "error event: {}: {:?}",
                error_event.message(),
                error_event.error()
            );
            on_event(WsEvent::Error(error_event.message()));
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    {
        let on_event = on_event.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            on_event(WsEvent::Opened);
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    {
        let onclose_callback = Closure::wrap(Box::new(move |_| {
            on_event(WsEvent::Closed);
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();
    }

    Ok(WsSender { ws: Some(ws) })
}
