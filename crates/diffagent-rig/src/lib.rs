//! Rig's persistent tool-using chat loop and explicit YAML flow executor.
//! The library sends observations to a channel; terminal rendering belongs to the binary.

mod agent;
mod chat;
mod tools;
mod workflows;

// Public API ------------------------------------------------------------------
pub use agent::RigRuntime;
pub use workflows::execute_flow;
