//! Shared CLI error codes, environment variable names, and binary discovery helpers.

use std::fmt;
use std::path::{Path, PathBuf};

/// Environment variable overriding the default `~/.diffagent` state directory.
pub(crate) const DIFFAGENT_HOME_ENV: &str = "DIFFAGENT_HOME";

/// Environment variable overriding the `diffagent-desktop` executable path.
pub(crate) const DIFFAGENT_DESKTOP_BIN_ENV: &str = "DIFFAGENT_DESKTOP_BIN";

/// Structured CLI failure mapped to stable JSON error codes at the process boundary.
#[derive(Debug)]
pub(crate) enum CliError {
    /// Invalid CLI flag, argument, or environment input.
    InvalidArgument(String),
    /// Requested executable or target resource does not exist.
    NotFound(String),
    /// Subprocess or runtime execution failed.
    Failed(String),
}

impl CliError {
    // Public ------------------------------------------------------------------

    /// Returns the wire error code rendered in `--json` error envelopes.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound(_) => "not_found",
            Self::Failed(_) => "command_failed",
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(msg) | Self::NotFound(msg) | Self::Failed(msg) => {
                f.write_str(msg)
            }
        }
    }
}

impl std::error::Error for CliError {}

/// Resolves the active `DIFFAGENT_HOME` path without creating or modifying directories on disk.
pub(crate) fn resolve_home_dir() -> Result<PathBuf, CliError> {
    if let Some(explicit) = std::env::var_os(DIFFAGENT_HOME_ENV).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(explicit));
    }
    let user_home = dirs::home_dir().ok_or_else(|| {
        CliError::Failed("Cannot resolve home directory from environment".to_string())
    })?;
    Ok(user_home.join(".diffagent"))
}

/// Resolves the `diffagent-desktop` executable path from `DIFFAGENT_DESKTOP_BIN` or standard locations.
pub(crate) fn resolve_desktop_executable() -> Result<PathBuf, CliError> {
    if let Some(override_bin) =
        std::env::var_os(DIFFAGENT_DESKTOP_BIN_ENV).filter(|v| !v.is_empty())
    {
        let candidate = PathBuf::from(override_bin);
        if is_executable_file(&candidate) {
            return Ok(candidate);
        }
        return Err(CliError::NotFound(format!(
            "Desktop executable not found: {}",
            candidate.display()
        )));
    }

    let binary_name = desktop_binary_name();
    let mut candidates = Vec::new();

    if let Ok(current_exe) = std::env::current_exe()
        && let Some(dir) = current_exe.parent()
    {
        candidates.push(dir.join(binary_name));
        if let Some(repo_root) = dir.parent().and_then(Path::parent) {
            candidates.push(
                repo_root
                    .join("apps/diffagent-desktop/target/debug")
                    .join(binary_name),
            );
            candidates.push(
                repo_root
                    .join("apps/diffagent-desktop/target/release")
                    .join(binary_name),
            );
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            candidates.push(dir.join(binary_name));
        }
    }

    #[cfg(target_os = "macos")]
    candidates.push(PathBuf::from(
        "/Applications/diffagent-desktop.app/Contents/MacOS/diffagent-desktop",
    ));

    for candidate in candidates {
        if is_executable_file(&candidate) {
            return Ok(candidate);
        }
    }

    Err(CliError::NotFound(
        "Desktop executable not found; build diffagent-desktop or set DIFFAGENT_DESKTOP_BIN"
            .to_string(),
    ))
}

fn desktop_binary_name() -> &'static str {
    if cfg!(windows) {
        "diffagent-desktop.exe"
    } else {
        "diffagent-desktop"
    }
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}
