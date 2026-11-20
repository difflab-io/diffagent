//! CLI `open` command launching the native Diffagent Desktop application detached.

use std::process::{Command, Stdio};

use crate::models::{
    CliError, OpenCommandOutput,
    common::{DIFFAGENT_HOME_ENV, resolve_desktop_executable, resolve_home_dir},
};

/// Resolves the `diffagent-desktop` binary and spawns it detached with stdio closed.
#[tracing::instrument]
pub(crate) fn run(json: bool) -> anyhow::Result<()> {
    let executable = resolve_desktop_executable()?;
    let home_dir = resolve_home_dir()?;

    Command::new(&executable)
        .env(DIFFAGENT_HOME_ENV, &home_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            CliError::Failed(format!(
                "Failed to launch Diffagent Desktop ({}): {err}",
                executable.display()
            ))
        })?;

    OpenCommandOutput::new(executable.to_string_lossy()).print(json);
    Ok(())
}
