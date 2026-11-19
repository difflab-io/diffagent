//! Tauri application state, lifecycle, Specta TypeScript binding generation, and IPC command handlers.

use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, State};

use crate::{
    error::Result,
    events::{AppEvent, AppEvents, AppReadyEvent},
};

/// Environment variable overriding the default `~/.diffagent` home directory.
pub const DIFFAGENT_HOME_ENV: &str = "DIFFAGENT_HOME";

/// Host metadata returned to the webview shell on startup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// Desktop package name (`diffagent-desktop`).
    pub name: String,
    /// Desktop package SemVer version.
    pub version: String,
    /// Resolved `DIFFAGENT_HOME` directory path.
    pub home_dir: String,
}

/// Unified Tauri host state shared across IPC command handlers.
#[derive(Clone)]
pub struct AppState {
    /// Active `DIFFAGENT_HOME` directory path.
    pub home_dir: PathBuf,
    /// In-process backend event bus and webview notification publisher.
    pub events: AppEvents,
    /// Tracks whether the webview shell completed startup registration.
    pub frontend_ready: Arc<AtomicBool>,
}

/// Builds a native Wry Tauri builder configured with host state and IPC handlers.
pub fn builder(explicit_home: Option<PathBuf>) -> tauri::Builder<tauri::Wry> {
    configure_builder(tauri::Builder::default(), explicit_home)
}

/// Configures any Tauri runtime builder (`Wry` or `MockRuntime`) with host state and IPC handlers.
pub fn configure_builder<R: tauri::Runtime>(
    builder: tauri::Builder<R>,
    explicit_home: Option<PathBuf>,
) -> tauri::Builder<R> {
    builder
        .setup(move |app| {
            init_app_state(app.handle(), explicit_home)?;
            Ok(())
        })
        .invoke_handler(commands)
}

pub(crate) fn init_app_state<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    explicit_home: Option<PathBuf>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if app_handle.try_state::<AppState>().is_some() {
        return Ok(());
    }
    let home_dir = resolve_home_dir(app_handle, explicit_home)?;
    let events = AppEvents::from_app_handle(app_handle);
    let handler_events = events.clone();
    events.set_handler(move |event| match event {
        AppEvent::FrontendReady(payload) => {
            handler_events.emit_to_webview("app-ready", &payload);
        }
    });
    app_handle.manage(AppState {
        home_dir,
        events,
        frontend_ready: Arc::new(AtomicBool::new(false)),
    });
    Ok(())
}

fn resolve_home_dir<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    explicit_home: Option<PathBuf>,
) -> std::result::Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(home) = explicit_home {
        return Ok(home);
    }
    if let Some(env_home) = std::env::var_os(DIFFAGENT_HOME_ENV).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(env_home));
    }
    let user_home = dirs::home_dir().or_else(|| app_handle.path().home_dir().ok());
    match user_home {
        Some(dir) => Ok(dir.join(".diffagent")),
        None => Err("Cannot resolve home directory".into()),
    }
}

/// Shared Tauri IPC command dispatch table for production and `MockRuntime` builds.
pub fn commands<R: tauri::Runtime>(invoke: tauri::ipc::Invoke<R>) -> bool {
    let handler: fn(tauri::ipc::Invoke<R>) -> bool =
        tauri::generate_handler![app_info, app_frontend_ready];
    handler(invoke)
}

/// Builds the `tauri-specta` command and type registry for exporting TypeScript bindings.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .disable_serde_phases()
        .dangerously_cast_bigints_to_number()
        .typ::<AppReadyEvent>()
        .commands(tauri_specta::collect_commands![
            app_info,
            app_frontend_ready
        ])
}

/// Exports TypeScript bindings (`apps/diffagent-web/src/lib/gen/bindings.ts`) from the Specta registry.
pub fn export_typescript_bindings_to(
    path: &std::path::Path,
) -> std::result::Result<String, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let exporter =
        specta_typescript::Typescript::default().header("// @ts-nocheck\n/* eslint-disable */\n");
    specta_builder()
        .export(exporter, path)
        .map_err(|err| err.to_string())?;
    std::fs::read_to_string(path).map_err(|err| err.to_string())
}

pub(crate) fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

/// Starts the native Tauri desktop application loop.
pub fn run() {
    builder(None)
        .run(app_context())
        .expect("error while running Tauri application");
}

/// Returns host package metadata and the active `DIFFAGENT_HOME` path.
#[tauri::command]
#[specta::specta]
pub fn app_info(state: State<'_, AppState>) -> Result<AppInfo> {
    Ok(AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        home_dir: state.home_dir.to_string_lossy().to_string(),
    })
}

/// Records that the frontend shell mounted and emits `app-ready` on the event bus.
#[tauri::command]
#[specta::specta]
pub fn app_frontend_ready(state: State<'_, AppState>) -> Result<()> {
    state.frontend_ready.store(true, Ordering::SeqCst);
    state
        .events
        .dispatch(AppEvent::FrontendReady(AppReadyEvent { ready: true }));
    Ok(())
}
