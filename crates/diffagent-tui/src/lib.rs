//! Backend-independent streaming terminal UI. Feed SDK-neutral
//! `diffagent_core::events::Event` values into [`run`] and receive user submissions
//! on its channel. Individual widgets can also consume [`UiEvent`] directly.
mod app;
mod editor;
mod markdown;
mod question_dialog;
mod sanitize;
mod terminal;
mod tool_activity;
mod transcript;
mod write_preview;

pub use app::{App, ToolKind, UiEvent};
pub use terminal::{ask_terminal, run, run_with_questions};
