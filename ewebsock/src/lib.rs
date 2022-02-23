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

#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::disallowed_method,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::missing_crate_level_docs
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

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
        let tx = std::sync::Mutex::new(tx);
        let on_event = std::sync::Arc::new(move |event| {
            wake_up(); // wake up UI thread
            if tx.lock().unwrap().send(event).is_ok() {
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

pub type EventHandler = std::sync::Arc<dyn Sync + Send + Fn(WsEvent) -> std::ops::ControlFlow<()>>;

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
    let sender = ws_connect(url.into(), on_event).unwrap();
    Ok((sender, receiver))
}
