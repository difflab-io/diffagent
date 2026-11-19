//! CLI view models and shared path/executable resolution helpers.

pub(crate) mod common;
pub(crate) mod open;
pub(crate) mod status;

pub(crate) use common::CliError;
pub(crate) use open::OpenCommandOutput;
pub(crate) use status::StatusCommandOutput;
