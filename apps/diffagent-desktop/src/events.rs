//! In-process backend event bus forwarding notifications to Tauri webviews and the stdio E2E bridge.

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Emitter;

type WebviewSink = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;
type EventHandler = Arc<dyn Fn(AppEvent) + Send + Sync>;

/// Payload emitted on `app-ready` when the webview finishes startup registration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppReadyEvent {
    /// Whether the webview shell reported readiness.
    pub ready: bool,
}

/// Typed desktop lifecycle events routed through [`AppEvents`].
#[derive(Clone, Debug)]
pub enum AppEvent {
    /// The frontend shell finished mounting and invoked `app_frontend_ready`.
    FrontendReady(AppReadyEvent),
}

/// In-process event bus coordinating backend handlers and outbound webview sinks.
#[derive(Clone, Default)]
pub struct AppEvents {
    webview_sinks: Arc<RwLock<Vec<WebviewSink>>>,
    handler: Arc<RwLock<Option<EventHandler>>>,
}

impl AppEvents {
    /// Creates an [`AppEvents`] bus bound to `app_handle`.
    pub(crate) fn from_app_handle<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) -> Self {
        let emit_handle = app_handle.clone();
        let events = Self::default();
        events.add_webview_sink(move |event, payload| {
            let _ = emit_handle.emit(event, payload);
        });
        events
    }

    // Public ------------------------------------------------------------------

    /// Registers the central backend event handler invoked on every [`AppEvents::dispatch`] call.
    pub fn set_handler<F>(&self, handler: F)
    where
        F: Fn(AppEvent) + Send + Sync + 'static,
    {
        if let Ok(mut slot) = self.handler.write() {
            *slot = Some(Arc::new(handler));
        }
    }

    /// Registers an outbound sink invoked whenever [`AppEvents::emit_to_webview`] publishes an event.
    pub fn add_webview_sink<F>(&self, sink: F)
    where
        F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        if let Ok(mut sinks) = self.webview_sinks.write() {
            sinks.push(Arc::new(sink));
        }
    }

    /// Dispatches a typed [`AppEvent`] to the registered backend handler.
    pub fn dispatch(&self, event: AppEvent) {
        let handler = self.handler.read().ok().and_then(|slot| slot.clone());
        if let Some(handler) = handler {
            handler(event);
        }
    }

    /// Serializes `payload` and forwards it to all registered webview sinks.
    pub fn emit_to_webview<T: serde::Serialize>(&self, event_name: &str, payload: &T) {
        if let Ok(value) = serde_json::to_value(payload)
            && let Ok(sinks) = self.webview_sinks.read()
        {
            for sink in sinks.iter() {
                sink(event_name, value.clone());
            }
        }
    }
}
