#![allow(trivial_casts)]

use std::{cell::RefCell, ops::ControlFlow, rc::Rc};
use wasm_bindgen::JsValue;

use crate::{EventHandler, Options, Result, WsEvent, WsMessage};

type BlobReadCallback = wasm_bindgen::closure::Closure<dyn FnMut(web_sys::ProgressEvent)>;

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
    socket: Option<Rc<web_sys::WebSocket>>,
}

impl Drop for WsSender {
    fn drop(&mut self) {
        self.close();
    }
}

impl WsSender {
    /// Send the message to the server.
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn send(&mut self, msg: WsMessage) {
        if let Some(socket) = &mut self.socket {
            let result = match msg {
                // `binaryType` controls how incoming messages are represented.
                // Sending bytes must not change it, or future receives can fall
                // back from `ArrayBuffer` to the slower `Blob` path.
                WsMessage::Binary(data) => socket.send_with_u8_array(&data),
                WsMessage::Text(text) => socket.send_with_str(&text),
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
    pub fn close(&mut self) {
        if let Some(socket) = self.socket.take() {
            close_socket(&socket);
        }
    }

    /// Forget about this sender without closing the connection.
    pub fn forget(mut self) {
        self.socket = None;
    }
}

pub(crate) fn ws_receive_impl(url: String, options: Options, on_event: EventHandler) -> Result<()> {
    ws_connect_impl(url, options, on_event).map(|sender| sender.forget())
}

#[allow(clippy::needless_pass_by_value)] // For consistency with the native version
pub(crate) fn ws_connect_impl(
    url: String,
    options: Options,
    on_event: EventHandler,
) -> Result<WsSender> {
    // Based on https://wasm-bindgen.github.io/wasm-bindgen/examples/websockets.html

    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast as _;

    // Connect to a server
    let socket =
        web_sys::WebSocket::new_with_str_sequence(&url, &JsValue::from(options.subprotocols))
            .map_err(string_from_js_value)?;
    let socket = Rc::new(socket);

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // Allow it to be shared by the different callbacks:
    let on_event: Rc<dyn Send + Fn(WsEvent) -> ControlFlow<()>> = on_event.into();

    // onmessage callback
    {
        let on_event = on_event.clone();
        let socket2 = socket.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            // Handle difference Text/Binary,...
            let control = if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                on_event(WsEvent::Message(WsMessage::Binary(array.to_vec())))
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                read_blob_message(&blob, &on_event, socket2.clone())
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                on_event(WsEvent::Message(WsMessage::Text(string_from_js_string(
                    txt,
                ))))
            } else {
                log::debug!("Unknown websocket message received: {:?}", e.data());
                on_event(WsEvent::Message(WsMessage::Unknown(string_from_js_value(
                    e.data(),
                ))))
            };
            if control.is_break() {
                close_socket(&socket2);
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        // set message event handler on WebSocket
        socket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        // forget the callback to keep it alive
        onmessage_callback.forget();
    }

    {
        let on_event = on_event.clone();
        let onerror_callback = Closure::wrap(Box::new(move |error_event: web_sys::ErrorEvent| {
            // using reflect instead of error_event.message() to avoid panic on null
            let message = js_sys::Reflect::get(&error_event, &"message".into()).unwrap_or_default();
            let error = js_sys::Reflect::get(&error_event, &"error".into()).unwrap_or_default();
            log::error!("error event: {message:?}: {error:?}");
            #[expect(
                unused_must_use,
                reason = "we intentionally ignore the return of `on_event`"
            )]
            on_event(WsEvent::Error(
                message
                    .as_string()
                    .unwrap_or_else(|| "Unknown error".to_owned()),
            ));
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        socket.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    {
        let socket2 = socket.clone();
        let on_event = on_event.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            let control = on_event(WsEvent::Opened);
            if control.is_break() {
                close_socket(&socket2);
            }
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        socket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    {
        let onclose_callback = Closure::wrap(Box::new(move |_| {
            #[expect(
                unused_must_use,
                reason = "we intentionally ignore the return of `on_event`"
            )]
            on_event(WsEvent::Closed);
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        socket.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();
    }

    Ok(WsSender {
        socket: Some(socket),
    })
}

fn read_blob_message(
    blob: &web_sys::Blob,
    on_event: &Rc<dyn Send + Fn(WsEvent) -> ControlFlow<()>>,
    socket: Rc<web_sys::WebSocket>,
) -> ControlFlow<()> {
    // A higher-level alternative to this FileReader plumbing is gloo-file:
    // https://crates.io/crates/gloo-file
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast as _;

    let file_reader = web_sys::FileReader::new().expect("Failed to create FileReader");
    let file_reader_clone = file_reader.clone();

    let cb_holder: Rc<RefCell<Option<BlobReadCallback>>> = Rc::new(RefCell::new(None));
    let cb_holder_clone = cb_holder.clone();
    let onloadend_on_event = on_event.clone();

    // `FileReader` is asynchronous, so the callback must stay alive until the
    // `loadend` event. The callback clears the JS handler and drops itself once
    // it has fired, avoiding a per-message closure leak on the Blob fallback.
    let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
        file_reader_clone.set_onloadend(None);

        let control = match file_reader_clone.result() {
            Ok(file_reader) => {
                let array = js_sys::Uint8Array::new(&file_reader);
                onloadend_on_event(WsEvent::Message(WsMessage::Binary(array.to_vec())))
            }
            Err(err) => onloadend_on_event(WsEvent::Error(format!(
                "Failed to read binary blob: {}",
                string_from_js_value(err)
            ))),
        };
        if control.is_break() {
            close_socket(&socket);
        }

        cb_holder_clone.borrow_mut().take();
    }) as Box<dyn FnMut(web_sys::ProgressEvent)>);

    file_reader.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
    *cb_holder.borrow_mut() = Some(onloadend_cb);

    if let Err(err) = file_reader.read_as_array_buffer(blob) {
        file_reader.set_onloadend(None);
        cb_holder.borrow_mut().take();
        on_event(WsEvent::Error(format!(
            "Failed to read binary blob: {}",
            string_from_js_value(err)
        )))
    } else {
        ControlFlow::Continue(())
    }
}

fn close_socket(socket: &web_sys::WebSocket) {
    if let Err(err) = socket.close() {
        log::warn!("Failed to close WebSocket: {}", string_from_js_value(err));
    } else {
        log::debug!("Closed WebSocket");
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    // The real Blob fallback depends on the browser's asynchronous `FileReader`.
    // This fake keeps the same shape but makes the result deterministic and
    // lets the test inspect whether Rust released the `onloadend` handler.
    #[wasm_bindgen(inline_js = r#"
        export function install_fake_file_reader() {
            globalThis.__ewebsockOriginalFileReader = globalThis.FileReader;
            globalThis.__ewebsockLastFileReader = undefined;
            globalThis.FileReader = class {
                constructor() {
                    this.onloadend = null;
                    this.result = null;
                    globalThis.__ewebsockLastFileReader = this;
                }

                readAsArrayBuffer(_blob) {
                    Promise.resolve().then(() => {
                        this.result = new Uint8Array([4, 5, 6]).buffer;
                        const onloadend = this.onloadend;
                        if (onloadend) {
                            onloadend.call(this, new ProgressEvent("loadend"));
                        }
                    });
                }
            };
        }

        export function restore_file_reader() {
            globalThis.FileReader = globalThis.__ewebsockOriginalFileReader;
            delete globalThis.__ewebsockOriginalFileReader;
            delete globalThis.__ewebsockLastFileReader;
        }

        export function last_file_reader_onloadend_is_null() {
            const reader = globalThis.__ewebsockLastFileReader;
            return !!reader && reader.onloadend == null;
        }

        export function next_tick() {
            return new Promise(resolve => setTimeout(resolve, 0));
        }
    "#)]
    extern "C" {
        fn install_fake_file_reader();
        fn restore_file_reader();
        fn last_file_reader_onloadend_is_null() -> bool;
        fn next_tick() -> js_sys::Promise;
    }

    thread_local! {
        // `EventHandler` is `Send` for API parity with native, but wasm tests run
        // on one browser thread. Thread-local storage keeps the test handler simple
        // without introducing synchronization primitives that do not matter here.
        static EVENTS: RefCell<Vec<WsEvent>> = const { RefCell::new(Vec::new()) };
    }

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn sending_binary_does_not_change_incoming_binary_type() {
        // No server needs to accept this connection. The bug was synchronous:
        // constructing `WsSender` around a browser WebSocket and calling
        // `send(Binary)` changed the socket's incoming binary representation.
        let socket = match web_sys::WebSocket::new("ws://127.0.0.1:1") {
            Ok(socket) => socket,
            Err(err) => panic!("failed to create websocket: {err:?}"),
        };
        socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let mut sender = WsSender {
            socket: Some(Rc::new(socket.clone())),
        };
        sender.send(WsMessage::Binary(vec![1, 2, 3]));

        // `binaryType` only controls incoming messages. A send must leave it
        // alone so future binary receives stay on the ArrayBuffer fast path.
        assert_eq!(socket.binary_type(), web_sys::BinaryType::Arraybuffer);

        sender.close();
    }

    #[wasm_bindgen_test]
    async fn blob_reader_clears_onloadend_after_dispatch() {
        install_fake_file_reader();

        // The socket is only used if the callback asks to close the connection;
        // this test keeps the handler on `Continue`, so no live server is needed.
        let socket = match web_sys::WebSocket::new("ws://127.0.0.1:1") {
            Ok(socket) => Rc::new(socket),
            Err(err) => panic!("failed to create websocket: {err:?}"),
        };
        EVENTS.with(|events| events.borrow_mut().clear());
        let on_event: Rc<dyn Send + Fn(WsEvent) -> ControlFlow<()>> = Rc::new(|event| {
            EVENTS.with(|events| events.borrow_mut().push(event));
            ControlFlow::Continue(())
        });
        let blob = match web_sys::Blob::new() {
            Ok(blob) => blob,
            Err(err) => panic!("failed to create blob: {err:?}"),
        };

        let control = read_blob_message(&blob, &on_event, socket.clone());
        assert!(control.is_continue());

        // Wait for the fake FileReader to schedule and fire `loadend`.
        if let Err(err) = JsFuture::from(next_tick()).await {
            panic!("failed to wait for fake FileReader: {err:?}");
        }

        // The handler slot must be cleared after `loadend`; otherwise the
        // FileReader and Rust closure can keep each other alive per Blob message.
        assert!(last_file_reader_onloadend_is_null());

        // The cleanup must not change behavior: the Blob is still delivered as
        // the original binary message payload.
        let events = EVENTS.with(|events| events.borrow().clone());
        match events.as_slice() {
            [WsEvent::Message(WsMessage::Binary(data))] => {
                assert_eq!(data.as_slice(), &[4, 5, 6]);
            }
            events => panic!("unexpected events: {events:?}"),
        }

        restore_file_reader();
        close_socket(&socket);
    }
}
