//! CLI view model for `diffagent open`.

use serde::Serialize;

/// Terminal and JSON output payload for `diffagent open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct OpenCommandOutput {
    /// Spawned `diffagent-desktop` executable path.
    pub(crate) executable: String,
}

impl OpenCommandOutput {
    /// Creates a new `open` command output payload.
    pub(crate) fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    // Public ------------------------------------------------------------------

    /// Prints the desktop launch confirmation as JSON or human-readable text on stdout.
    pub(crate) fn print(&self, json: bool) {
        if json {
            println!(
                "{}",
                serde_json::to_string(self).expect("serialize OpenCommandOutput")
            );
        } else {
            println!("Opened Diffagent Desktop ({})", self.executable);
        }
    }
}
