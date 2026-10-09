//! Ax chat and explicit YAML flow execution. No UI dependency is used here.
#![allow(clippy::result_large_err)] // Ax tool handlers require AxResult<Value>.

mod agent;
mod chat;
mod tools;
mod workflows;

// Public API ------------------------------------------------------------------
pub use agent::Engine;
