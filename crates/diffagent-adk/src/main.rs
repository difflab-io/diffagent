use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use diffagent_adk::{Chat, run_flow_with_question_handler};
use diffagent_core::{
    events::{Event, QuestionAnswer, QuestionHandler, QuestionService},
    spec::LoadedAgent,
};
use std::{
    collections::BTreeMap,
    io::IsTerminal,
    path::PathBuf,
    sync::{Arc, mpsc},
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
    let agent = LoadedAgent::load(&agent_path)?;
    let runtime = tokio::runtime::Runtime::new()?;
    if let Some(name) = name {
        let (tx, rx) = mpsc::channel();
        let printer = std::thread::spawn(move || {
            for event in rx {
                if let Ok(line) = serde_json::to_string(&event) {
                    println!("{line}");
                }
            }
        });
        let handler: QuestionHandler = Arc::new(|q: &str, opts: &[String]| {
            if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
                diffagent_tui::ask_terminal(q, opts)
            } else {
                QuestionAnswer::Skipped {
                    reason: "non-interactive terminal".into(),
                }
            }
        });
        let result = runtime.block_on(run_flow_with_question_handler(
            agent, &workspace, &name, &params, &tx, handler,
        ));
        drop(tx);
        let _ = printer.join();
        result?;
    } else {
        let (tx, rx) = mpsc::channel();
        let questions = QuestionService::new(tx.clone());
        let service = questions.clone();
        let handler: QuestionHandler =
            Arc::new(move |q: &str, opts: &[String]| service.ask(q, opts));
        let (submissions_tx, submissions_rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            runtime.block_on(async move {
                match Chat::builder(agent, &workspace, tx.clone())
                    .with_question_handler(handler)
                    .build()
                    .await
                {
                    Ok(chat) => {
                        while let Ok(prompt) = submissions_rx.recv() {
                            if let Err(error) = chat.submit(&prompt).await {
                                let _ = tx.send(Event::Error {
                                    message: error.to_string(),
                                });
                            }
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Event::Error {
                            message: error.to_string(),
                        });
                    }
                }
            });
        });
        diffagent_tui::run_with_questions(rx, submissions_tx, questions)?;
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
