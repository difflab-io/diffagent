//! Desktop host services.

#[cfg(any(test, feature = "e2e-bridge"))]
pub mod ipc_bridge;

#[cfg(any(test, feature = "e2e-bridge"))]
pub use ipc_bridge::IpcBridgeService;
