use std::{
    collections::BTreeMap,
    io::IsTerminal,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
};

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use diffagent_core::{
    events::{QuestionAnswer, QuestionHandler, QuestionService},
    spec::LoadedAgent,
    tools::ToolHost,
};
use diffagent_rig::RigRuntime;

// Types and structs -----------------------------------------------------------

#[derive(Parser)]
struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Paths {
    #[arg(long)]
    agent: PathBuf,
    #[arg(long)]
    workspace: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    Chat {
        #[command(flatten)]
        paths: Paths,
    },
    Flow {
        name: String,
        #[command(flatten)]
        paths: Paths,
        #[arg(long = "arg", value_parser = parse_arg)]
        params: Vec<(String, String)>,
    },
}

// Public API ------------------------------------------------------------------

fn main() -> Result<()> {
    let (agent_path, workspace, name, params) = match Options::parse().command {
        Command::Chat { paths } => (paths.agent, paths.workspace, None, BTreeMap::new()),
        Command::Flow {
            name,
            paths,
            params,
        } => (
            paths.agent,
            paths.workspace,
            Some(name),
            flow_params(params)?,
        ),
    };
    let is_chat = name.is_none();
    let agent = LoadedAgent::load(&agent_path)?;
    let host = ToolHost::new(&workspace, agent.spec.tools.clone())?;
    let (events_tx, events_rx) = mpsc::channel();
    let questions = QuestionService::new(events_tx.clone());
    let handler = if is_chat {
        let questions = questions.clone();
        Arc::new(move |q: &str, opts: &[String]| questions.ask(q, opts)) as QuestionHandler
    } else {
        Arc::new(|q: &str, opts: &[String]| {
            if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
                diffagent_tui::ask_terminal(q, opts)
            } else {
                QuestionAnswer::Skipped {
                    reason: "non-interactive terminal".into(),
                }
            }
        }) as QuestionHandler
    };
    let runtime = RigRuntime::new(agent, host, events_tx.clone()).with_question_handler(handler);
    drop(events_tx);
    if is_chat {
        let (input_tx, input_rx) = mpsc::channel::<String>();
        let worker = thread::spawn(move || -> Result<()> {
            let rt = tokio::runtime::Runtime::new()?;
            let mut history = Vec::new();
            while let Ok(prompt) = input_rx.recv() {
                if let Err(error) = rt.block_on(runtime.chat(&prompt, &mut history)) {
                    runtime_error(&runtime, error);
                }
            }
            Ok(())
        });
        diffagent_tui::run_with_questions(events_rx, input_tx, questions)?;
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("chat worker panicked"))??;
    } else {
        // Drain on a separate thread so events appear while a slow model or task runs.
        let printer = thread::spawn(move || {
            while let Ok(event) = events_rx.recv() {
                if let Ok(line) = serde_json::to_string(&event) {
                    println!("{line}");
                }
            }
        });
        let rt = tokio::runtime::Runtime::new()?;
        let name = name.ok_or_else(|| anyhow::anyhow!("flow name missing"))?;
        let result = rt.block_on(runtime.flow(&name, params));
        drop(runtime);
        let _ = printer.join();
        result?;
    }
    Ok(())
}

// Helpers ---------------------------------------------------------------------

fn parse_arg(raw: &str) -> std::result::Result<(String, String), String> {
    let (key, value) = raw.split_once('=').ok_or("--arg requires key=value")?;
    if key.is_empty() {
        return Err("--arg requires a nonempty key".into());
    }
    Ok((key.into(), value.into()))
}

fn flow_params(params: Vec<(String, String)>) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (key, value) in params {
        if values.insert(key.clone(), value).is_some() {
            bail!("duplicate flow argument: {key}");
        }
    }
    Ok(values)
}

fn runtime_error(runtime: &RigRuntime, error: anyhow::Error) {
    runtime.emit_error(error.to_string());
}

// Tests -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_and_flow_without_running_a_model() -> Result<()> {
        let chat = Options::try_parse_from(["cli", "chat", "--agent", "a", "--workspace", "w"])?;
        assert!(matches!(chat.command, Command::Chat { .. }));
        let flow = Options::try_parse_from([
            "cli",
            "flow",
            "implement",
            "--agent",
            "a",
            "--workspace",
            "w",
            "--arg",
            "request=x=y",
        ])?;
        let Command::Flow { name, params, .. } = flow.command else {
            panic!("expected flow");
        };
        assert_eq!(name, "implement");
        assert_eq!(flow_params(params)?["request"], "x=y");
        assert!(
            Options::try_parse_from([
                "cli",
                "chat",
                "--agent",
                "a",
                "--workspace",
                "w",
                "--arg",
                "x=y"
            ])
            .is_err()
        );
        assert!(flow_params(vec![("x".into(), "1".into()), ("x".into(), "2".into())]).is_err());
        Ok(())
    }
}
