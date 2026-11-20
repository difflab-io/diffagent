//! Diffagent command-line client and desktop application launcher.
#![warn(missing_docs, unreachable_pub)]

use clap::{CommandFactory, Parser, Subcommand, error::ErrorKind};

mod commands;
mod models;

use models::CliError;

#[derive(Subcommand, Debug)]
enum Command {
    /// Print the Diffagent CLI version and resolved home directory.
    Status,
    /// Launch the Diffagent Desktop application.
    Open,
}

/// Command-line client and desktop launcher for Diffagent.
#[derive(Parser, Debug)]
#[command(name = "diffagent", version, about)]
struct Cli {
    /// Return machine-readable JSON on stdout, with diagnostics on stderr.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

fn main() {
    init_tracing();
    let json_flag = std::env::args().any(|arg| arg == "--json");

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{err}");
                return;
            }
            _ => {
                emit_error(
                    &CliError::InvalidArgument(err.to_string().trim_end().to_string()),
                    json_flag,
                );
                std::process::exit(2);
            }
        },
    };

    if let Err(error) = dispatch(cli.command, cli.json) {
        let cli_err = error
            .downcast::<CliError>()
            .unwrap_or_else(|other| CliError::Failed(format!("{other:#}")));
        emit_error(&cli_err, cli.json);
        std::process::exit(1);
    }
}

fn dispatch(command: Option<Command>, json: bool) -> anyhow::Result<()> {
    match command {
        None => {
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            Ok(())
        }
        Some(Command::Status) => commands::status::run(json),
        Some(Command::Open) => commands::open::run(json),
    }
}

fn emit_error(error: &CliError, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "error": {
                    "code": error.code(),
                    "message": error.to_string(),
                }
            })
        );
    } else {
        eprintln!("{error}");
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
