//! CLI view model for `diffagent status`.

use serde::Serialize;

/// Terminal and JSON output payload for `diffagent status`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct StatusCommandOutput {
    /// Active `diffagent` CLI SemVer string.
    pub(crate) version: String,
    /// Resolved `DIFFAGENT_HOME` directory path.
    pub(crate) home_dir: String,
}

impl StatusCommandOutput {
    /// Creates a new status response payload.
    pub(crate) fn new(version: impl Into<String>, home_dir: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            home_dir: home_dir.into(),
        }
    }

    // Public ------------------------------------------------------------------

    /// Prints the status summary as JSON or human-readable text on stdout.
    pub(crate) fn print(&self, json: bool) {
        if json {
            println!(
                "{}",
                serde_json::to_string(self).expect("serialize StatusCommandOutput")
            );
        } else {
            println!("diffagent {} (home: {})", self.version, self.home_dir);
        }
    }
}
