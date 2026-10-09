use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use diffagent_ax::Engine;
use diffagent_core::events::{Event, QuestionAnswer, QuestionHandler, QuestionService};
use std::{
    collections::BTreeMap,
    io::IsTerminal,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
};

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
    let (agent, workspace, name, arguments) = match Options::parse().command {
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
    let (events_tx, events_rx) = mpsc::channel();
    let questions = QuestionService::new(events_tx.clone());
    let handler = if name.is_some() {
        Arc::new(|q: &str, opts: &[String]| {
            if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
                diffagent_tui::ask_terminal(q, opts)
            } else {
                QuestionAnswer::Skipped {
                    reason: "non-interactive terminal".into(),
                }
            }
        }) as QuestionHandler
    } else {
        let questions = questions.clone();
        Arc::new(move |q: &str, opts: &[String]| questions.ask(q, opts)) as QuestionHandler
    };
    let engine =
        Engine::load(&agent, &workspace, events_tx.clone())?.with_question_handler(handler);
    if let Some(name) = name {
        let printer = thread::spawn(move || {
            while let Ok(event) = events_rx.recv() {
                println!("{}", serde_json::to_string(&event).unwrap_or_default());
            }
        });
        let result = engine.run_flow(&name, &arguments);
        if let Err(error) = &result {
            let _ = events_tx.send(Event::Error {
                message: error.to_string(),
            });
        }
        drop(engine);
        drop(events_tx);
        let _ = printer.join();
        result?;
    } else {
        let (input_tx, input_rx) = mpsc::channel::<String>();
        thread::spawn(move || {
            let mut history = Vec::new();
            while let Ok(request) = input_rx.recv() {
                if let Err(error) = engine.chat(&mut history, &request) {
                    let _ = events_tx.send(Event::Error {
                        message: error.to_string(),
                    });
                }
            }
        });
        diffagent_tui::run_with_questions(events_rx, input_tx, questions)?;
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
