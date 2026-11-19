//! CLI `status` command reporting version and resolved home directory without mutating disk.

use crate::models::{StatusCommandOutput, common::resolve_home_dir};

/// Resolves and prints the CLI version and `DIFFAGENT_HOME` directory.
#[tracing::instrument]
pub(crate) fn run(json: bool) -> anyhow::Result<()> {
    let home_dir = resolve_home_dir()?;
    StatusCommandOutput::new(env!("CARGO_PKG_VERSION"), home_dir.to_string_lossy()).print(json);
    Ok(())
}
