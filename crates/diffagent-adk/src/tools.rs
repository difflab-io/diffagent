use adk_tool::FunctionTool;
use diffagent_core::{
    events::{Event, QuestionAnswer, QuestionHandler},
    spec::LoadedAgent,
    tools::{ToolError, ToolHost},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, mpsc::Sender},
};

use crate::workflows::execute_flow_with_question_handler;

// Types and structs -----------------------------------------------------------

#[derive(Clone, JsonSchema, Serialize, Deserialize)]
struct PathArg {
    path: String,
}
#[derive(Clone, JsonSchema, Serialize, Deserialize)]
struct WriteArg {
    path: String,
    content: String,
}
#[derive(Clone, JsonSchema, Serialize, Deserialize)]
struct TaskArg {
    task: String,
}
#[derive(Clone, JsonSchema, Serialize, Deserialize)]
struct MiseTaskArg {
    name: String,
}
#[derive(Clone, JsonSchema, Serialize, Deserialize)]
struct FlowArg {
    name: String,
    #[serde(default)]
    args: BTreeMap<String, String>,
}
#[derive(Clone, JsonSchema, Serialize, Deserialize)]
struct QuestionArg {
    question: String,
    options: Vec<String>,
}

// Public API ------------------------------------------------------------------

pub(crate) fn define_tools(
    agent: &Arc<LoadedAgent>,
    host: ToolHost,
    tx: &Sender<Event>,
    allow_flow: bool,
    question_handler: QuestionHandler,
    allow_questions: bool,
) -> Vec<Arc<dyn adk_core::Tool>> {
    let mut tools = Vec::new();
    let question_tx = tx.clone();
    let question_handler_for_flows = question_handler.clone();
    tools.push(define_tool(
        FunctionTool::new("ask_user_question", "Ask one question. Pass options: [] for free text, or one to four choices. A skipped response is not an answer.", move |_, args: Value| {
            let (tx, handler) = (question_tx.clone(), question_handler.clone());
            async move {
                let arg: QuestionArg = serde_json::from_value(args).map_err(tool_error)?;
                if arg.question.trim().is_empty() || arg.question.len() > 4096 || arg.options.len() > 4 || arg.options.iter().any(|option| option.trim().is_empty() || option.len() > 256) {
                    return Err(tool_error("question must be nonempty, at most 4096 bytes, with at most four nonempty options of 256 bytes each"));
                }
                let id = format!("question:{}", arg.question);
                emit(&tx, Event::ToolStarted { id: id.clone(), name: "ask_user_question".into(), detail: arg.question.clone() });
                let answer = if allow_questions {
                    tokio::task::spawn_blocking(move || handler(&arg.question, &arg.options))
                        .await.map_err(tool_error)
                } else {
                    Ok(QuestionAnswer::Skipped { reason: "questions disabled by flow policy".into() })
                };
                let summary = answer.as_ref().ok().map(|value| value.summary().to_owned());
                let result = answer.and_then(|value| serde_json::to_value(value).map_err(tool_error));
                emit(&tx, Event::ToolFinished {
                    id,
                    success: result.is_ok(),
                    output: match (&summary, &result) {
                        (Some(summary), Ok(_)) => summary.clone(),
                        (_, Err(error)) => error.to_string(),
                        _ => String::new(),
                    },
                });
                result
            }
        }).with_parameters_schema::<QuestionArg>(),
    ));
    let (reader, sender) = (host.clone(), tx.clone());
    tools.push(define_tool(
        FunctionTool::new(
            "read_file",
            "Read an allowlisted workspace file",
            move |_, args: Value| {
                let (host, tx) = (reader.clone(), sender.clone());
                async move {
                    let arg: PathArg = serde_json::from_value(args).map_err(tool_error)?;
                    let id = format!("read:{}", arg.path);
                    emit(
                        &tx,
                        Event::ToolStarted {
                            id: id.clone(),
                            name: "read_file".into(),
                            detail: arg.path.clone(),
                        },
                    );
                    let result = host.read_file(&arg.path);
                    emit(
                        &tx,
                        Event::ToolFinished {
                            id,
                            success: result.is_ok(),
                            output: result
                                .as_ref()
                                .map(|_| "read file".to_string())
                                .unwrap_or_else(|e| e.to_string()),
                        },
                    );
                    result
                        .map(|content| json!({"content": content}))
                        .map_err(tool_error)
                }
            },
        )
        .with_parameters_schema::<PathArg>()
        .with_read_only(true),
    ));
    let (writer, sender) = (host.clone(), tx.clone());
    tools.push(define_tool(
        FunctionTool::new(
            "write_file",
            "Atomically write complete content to an allowlisted workspace file",
            move |_, args: Value| {
                let (host, tx) = (writer.clone(), sender.clone());
                async move {
                    let arg: WriteArg = serde_json::from_value(args).map_err(tool_error)?;
                    let id = format!("write:{}", arg.path);
                    emit(
                        &tx,
                        Event::ToolStarted {
                            id: id.clone(),
                            name: "write_file".into(),
                            detail: arg.path.clone(),
                        },
                    );
                    let result = host.write_file(&arg.path, &arg.content, |proposal| {
                        let content = std::str::from_utf8(proposal.content)
                            .map_err(|e| ToolError::Preview(e.to_string()))?;
                        emit(
                            &tx,
                            Event::WritePreview {
                                id: id.clone(),
                                path: proposal.relative.into(),
                                original: proposal.original.map(str::to_owned),
                                content: content.into(),
                            },
                        );
                        Ok(())
                    });
                    if result.is_ok() {
                        emit(
                            &tx,
                            Event::WriteCommitted {
                                id: id.clone(),
                                path: arg.path,
                            },
                        );
                    }
                    emit(
                        &tx,
                        Event::ToolFinished {
                            id,
                            success: result.is_ok(),
                            output: result
                                .as_ref()
                                .map(|r| format!("committed {} bytes", r.bytes))
                                .unwrap_or_else(|e| e.to_string()),
                        },
                    );
                    result
                        .map(|r| json!({"path":r.relative,"bytes":r.bytes}))
                        .map_err(tool_error)
                }
            },
        )
        .with_parameters_schema::<WriteArg>(),
    ));
    let (lister, sender) = (host.clone(), tx.clone());
    tools.push(define_tool(
        FunctionTool::new(
            "list_files",
            "List readable files in the workspace",
            move |_, _: Value| {
                let (host, tx) = (lister.clone(), sender.clone());
                async move {
                    let id = "list_files".to_string();
                    emit(
                        &tx,
                        Event::ToolStarted {
                            id: id.clone(),
                            name: id.clone(),
                            detail: String::new(),
                        },
                    );
                    let result = host.list_files();
                    emit(
                        &tx,
                        Event::ToolFinished {
                            id,
                            success: result.is_ok(),
                            output: result
                                .as_ref()
                                .map(|files| format!("{} files", files.len()))
                                .unwrap_or_else(|e| e.to_string()),
                        },
                    );
                    result
                        .map(|files| json!({"files":files}))
                        .map_err(tool_error)
                }
            },
        )
        .with_read_only(true),
    ));
    let (tasker, sender) = (host.clone(), tx.clone());
    let task_description = format!(
        "Run a configured named task. Available tasks: {}",
        agent
            .spec
            .tools
            .tasks
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    let task_tool = FunctionTool::new("run_task", task_description, move |_, args: Value| {
        let (host, tx) = (tasker.clone(), sender.clone());
        async move {
            let arg: TaskArg = serde_json::from_value(args).map_err(tool_error)?;
            let id = format!("task:{}", arg.task);
            emit(
                &tx,
                Event::ToolStarted {
                    id: id.clone(),
                    name: "run_task".into(),
                    detail: arg.task.clone(),
                },
            );
            let result = tokio::task::spawn_blocking(move || host.run_named_task(&arg.task))
                .await
                .map_err(tool_error)?
                .map_err(tool_error);
            emit(
                &tx,
                Event::ToolFinished {
                    id,
                    success: result.as_ref().is_ok_and(|output| output.success),
                    output: result
                        .as_ref()
                        .map(|output| output.output.clone())
                        .unwrap_or_else(|error| error.to_string()),
                },
            );
            result.map(|output| {
                json!({
                    "success": output.success,
                    "exit_code": output.exit_code,
                    "output": output.output,
                    "truncated": output.truncated,
                })
            })
        }
    })
    .with_parameters_schema::<TaskArg>();
    tools.push(define_tool(task_tool));
    if agent.spec.tools.mise_tasks {
        let (mise_lister, sender) = (host.clone(), tx.clone());
        tools.push(define_tool(
        FunctionTool::new(
            "list_mise_tasks",
            "List all mise tasks visible in the workspace, including tasks authored in mise.toml",
            move |_, _: Value| {
                let (host, tx) = (mise_lister.clone(), sender.clone());
                async move {
                    let id = "list_mise_tasks".to_string();
                    emit(
                        &tx,
                        Event::ToolStarted {
                            id: id.clone(),
                            name: "list_mise_tasks".into(),
                            detail: "workspace".into(),
                        },
                    );
                    let result = tokio::task::spawn_blocking(move || host.list_mise_tasks())
                        .await
                        .map_err(tool_error)
                        .and_then(|result| result.map_err(tool_error));
                    emit(
                        &tx,
                        Event::ToolFinished {
                            id,
                            success: result.is_ok(),
                            output: result.as_ref().map_or_else(
                                |error| error.to_string(),
                                |tasks| {
                                    bounded_output(&if tasks.is_empty() {
                                        "No mise tasks".into()
                                    } else {
                                        tasks.join("\n")
                                    })
                                },
                            ),
                        },
                    );
                    result.map(|tasks| json!({"tasks": tasks}))
                }
            },
        )
        .with_read_only(true),
    ));
        let (mise_runner, sender) = (host.clone(), tx.clone());
        tools.push(define_tool(
        FunctionTool::new(
            "run_mise_task",
            "Run a workspace mise task by name; use list_mise_tasks to discover names, not arbitrary shell commands",
            move |_, args: Value| {
                let (host, tx) = (mise_runner.clone(), sender.clone());
                async move {
                    let arg: MiseTaskArg = serde_json::from_value(args).map_err(tool_error)?;
                    let id = format!("mise:{}", arg.name);
                    emit(&tx, Event::ToolStarted {
                        id: id.clone(), name: "run_mise_task".into(), detail: arg.name.clone(),
                    });
                    let result = tokio::task::spawn_blocking(move || host.run_mise_task(&arg.name))
                        .await.map_err(tool_error).and_then(|result| result.map_err(tool_error));
                    emit(&tx, Event::ToolFinished {
                        id,
                        success: result.as_ref().is_ok_and(|output| output.success),
                        output: result.as_ref().map_or_else(|error| error.to_string(), |output| {
                            bounded_output(&format!("{} (exit {:?}, truncated={}):\n{}",
                                if output.success { "PASS" } else { "FAIL" },
                                output.exit_code, output.truncated, output.output))
                        }),
                    });
                    result.map(|output| json!({
                        "success": output.success, "exit_code": output.exit_code,
                        "output": output.output, "truncated": output.truncated,
                    }))
                }
            },
        ).with_parameters_schema::<MiseTaskArg>(),
    ));
    }
    if allow_flow && !agent.flows.is_empty() {
        let flow_description = format!(
            "Execute a configured YAML flow by name with parameters. Available flows: {}",
            agent.flows.keys().cloned().collect::<Vec<_>>().join(", ")
        );
        let (agent, host, sender, handler) = (
            agent.clone(),
            host.clone(),
            tx.clone(),
            question_handler_for_flows.clone(),
        );
        let flow_tool = FunctionTool::new("run_flow", flow_description, move |_, args: Value| {
            let (agent, host, tx, handler) =
                (agent.clone(), host.clone(), sender.clone(), handler.clone());
            async move {
                let arg: FlowArg = serde_json::from_value(args).map_err(tool_error)?;
                let id = format!("flow:{}", arg.name);
                emit(
                    &tx,
                    Event::ToolStarted {
                        id: id.clone(),
                        name: "run_flow".into(),
                        detail: arg.name.clone(),
                    },
                );
                let result = execute_flow_with_question_handler(
                    &agent, host, &arg.name, &arg.args, &tx, handler,
                )
                .await;
                emit(
                    &tx,
                    Event::ToolFinished {
                        id,
                        success: result.is_ok(),
                        output: result
                            .as_ref()
                            .map_or_else(|error| error.to_string(), Clone::clone),
                    },
                );
                result
                    .map(|output| json!({"output": output}))
                    .map_err(tool_error)
            }
        })
        .with_parameters_schema::<FlowArg>();
        tools.push(define_tool(flow_tool));
    }
    tools
}

// Helpers ---------------------------------------------------------------------

pub(crate) fn emit(tx: &Sender<Event>, event: Event) {
    let _ = tx.send(event);
}

fn tool_error(error: impl std::fmt::Display) -> adk_core::AdkError {
    adk_core::AdkError::tool(error.to_string())
}

fn define_tool(tool: FunctionTool) -> Arc<dyn adk_core::Tool> {
    Arc::new(tool)
}

fn bounded_output(text: &str) -> String {
    text.chars().take(12_000).collect()
}
