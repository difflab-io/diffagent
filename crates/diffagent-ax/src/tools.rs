use crate::Engine;
use anyhow::Result;
use axllm::{AxError, FieldType, Tool, tool};
use diffagent_core::events::{Event, QuestionAnswer};
use serde_json::{Value, json};
use std::collections::BTreeMap;

// Constants -------------------------------------------------------------------
const MAX_TURN_CHARS: usize = 12_000;

// Public API ------------------------------------------------------------------
impl Engine {
    pub(crate) fn define_tools(&self, flow_node: bool, allow_questions: bool) -> Vec<Tool> {
        let reader = self.clone();
        let read = tool("read_file")
            .description("Read an allowlisted relative workspace file")
            .arg("path", FieldType::string())
            .handler(move |args| {
                let path = arg(&args, "path")?;
                reader.activity("read_file", path, || {
                    reader
                        .host
                        .read_file(path)
                        .map(|s| json!({"content":s}))
                        .map_err(Into::into)
                })
            });
        let writer = self.clone();
        let write = tool("write_file")
            .description("Write complete content to an allowlisted relative file")
            .arg("path", FieldType::string())
            .arg("content", FieldType::string())
            .handler(move |args| {
                let path = arg(&args, "path")?;
                let content = arg(&args, "content")?;
                writer.activity("write_file", path, || {
                    let id = format!("write:{path}");
                    let receipt = writer.host.write_file(path, content, |proposal| {
                        writer.emit(Event::WritePreview {
                            id: id.clone(),
                            path: path.into(),
                            original: proposal.original.map(str::to_owned),
                            content: String::from_utf8_lossy(proposal.content).into_owned(),
                        });
                        Ok(())
                    })?;
                    writer.emit(Event::WriteCommitted {
                        id,
                        path: path.into(),
                    });
                    Ok(json!({"path":receipt.relative, "bytes":receipt.bytes}))
                })
            });
        let lister = self.clone();
        let list = tool("list_files")
            .description("List allowlisted workspace files")
            .handler(move |_| {
                lister.activity("list_files", "", || {
                    Ok(json!({"files":lister.host.list_files()?}))
                })
            });
        let runner = self.clone();
        let task_description = format!(
            "Run a configured named task. Available tasks: {}",
            self.agent
                .spec
                .tools
                .tasks
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        let task = tool("run_task").description(task_description)
            .arg("task", FieldType::string()).handler(move |args| {
                let name = arg(&args, "task")?;
                runner.activity("run_task", name, || {
                    let output = runner.host.run_named_task(name)?;
                    Ok(json!({"success":output.success,"exit_code":output.exit_code,"output":output.output,"truncated":output.truncated}))
                })
            });
        let mise_lister = self.clone();
        let list_mise_tasks = tool("list_mise_tasks")
            .description(
                "List all mise tasks visible in the workspace, including tasks in mise.toml",
            )
            .handler(move |_| {
                mise_lister.activity("list_mise_tasks", "workspace", || {
                    Ok(json!({"tasks":mise_lister.host.list_mise_tasks()?}))
                })
            });
        let mise_runner = self.clone();
        let run_mise_task = tool("run_mise_task")
            .description("Run a workspace mise task by name; use list_mise_tasks to discover names, not arbitrary shell commands")
            .arg("name", FieldType::string())
            .handler(move |args| {
                let name = arg(&args, "name")?;
                mise_runner.activity("run_mise_task", name, || {
                    let output = mise_runner.host.run_mise_task(name)?;
                    Ok(json!({"success":output.success,"exit_code":output.exit_code,"output":output.output,"truncated":output.truncated}))
                })
            });
        let asker = self.clone();
        let ask = tool("ask_user_question")
            .description("Ask one question. Pass options: [] for free text, or one to four choices. A skipped response is not an answer.")
            .arg("question", FieldType::string())
            .arg("options", FieldType::new("json"))
            .handler(move |args| {
                let question = arg(&args, "question")?;
                let options = args.get("options").and_then(Value::as_array)
                    .ok_or_else(|| AxError::runtime("options must be an array of strings"))?;
                if question.trim().is_empty() || question.len() > 4096 || options.len() > 4 {
                    return Err(AxError::runtime("question must be nonempty, at most 4096 bytes, with at most four options"));
                }
                let options: Vec<String> = options.iter().map(|v| v.as_str().map(str::to_owned)
                    .ok_or_else(|| AxError::runtime("options must be strings"))).collect::<axllm::AxResult<_>>()?;
                if options.iter().any(|option| option.trim().is_empty() || option.len() > 256) {
                    return Err(AxError::runtime("options must be nonempty and at most 256 bytes each"));
                }
                let id = format!("ask_user_question:{question}");
                asker.emit(Event::ToolStarted {
                    id: id.clone(),
                    name: "ask_user_question".into(),
                    detail: question.into(),
                });
                let answer = if allow_questions {
                    (asker.question_handler)(question, &options)
                } else {
                    QuestionAnswer::Skipped { reason: "questions disabled by flow policy".into() }
                };
                let summary = answer.summary();
                let result = serde_json::to_value(answer).map_err(|error| AxError::runtime(error.to_string()));
                asker.emit(Event::ToolFinished {
                    id,
                    success: result.is_ok(),
                    output: result.as_ref().map_or_else(|error| error.to_string(), |_| summary.into()),
                });
                result
            });
        let mut tools = vec![read, write, list];
        if !flow_node {
            tools.push(task);
        }
        if self.agent.spec.tools.mise_tasks {
            tools.extend([list_mise_tasks, run_mise_task]);
        }
        if !flow_node {
            tools.push(self.define_tool());
        }
        tools.push(ask);
        tools
    }

    /// Build the Ax-native YAML-flow tool, including its schema, argument
    /// validation and execution handler. Only chat exposes this tool; nodes
    /// cannot recursively invoke flows through the native-tool path.
    fn define_tool(&self) -> Tool {
        let flows = self.clone();
        let flow_description = format!(
            "Explicitly run a configured YAML flow by name with string arguments. Available flows: {}",
            self.agent
                .flows
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        tool("run_flow")
            .description(flow_description)
            .arg("name", FieldType::string())
            .arg("arguments", FieldType::new("json"))
            .handler(move |args| {
                let name = arg(&args, "name")?;
                let values = args
                    .get("arguments")
                    .and_then(Value::as_object)
                    .ok_or_else(|| AxError::runtime("arguments must be an object"))?;
                let mut parameters = BTreeMap::new();
                for (key, value) in values {
                    parameters.insert(
                        key.clone(),
                        value
                            .as_str()
                            .ok_or_else(|| AxError::runtime("flow arguments must be strings"))?
                            .to_owned(),
                    );
                }
                flows.activity("run_flow", name, || {
                    Ok(json!({"output": flows.define_flow(name, &parameters, 1)?}))
                })
            })
    }
}

// Helpers ---------------------------------------------------------------------
impl Engine {
    fn activity(
        &self,
        name: &str,
        detail: &str,
        action: impl FnOnce() -> Result<Value>,
    ) -> axllm::AxResult<Value> {
        let id = format!("{name}:{detail}");
        self.emit(Event::ToolStarted {
            id: id.clone(),
            name: name.into(),
            detail: detail.into(),
        });
        let result = action();
        self.emit(Event::ToolFinished {
            id,
            success: result.as_ref().is_ok_and(|value| {
                value
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            }),
            output: match &result {
                Ok(v) => bounded(&tool_output(name, v)),
                Err(e) => e.to_string(),
            },
        });
        result.map_err(|e| AxError::runtime(e.to_string()))
    }
}

pub(crate) fn bounded(s: &str) -> String {
    s.chars().take(MAX_TURN_CHARS).collect()
}

/// The SDK needs structured tool results, but the chat transcript needs readable text.
fn tool_output(name: &str, value: &Value) -> String {
    match name {
        "read_file" => value
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string()),
        "write_file" => match (
            value.get("path").and_then(Value::as_str),
            value.get("bytes").and_then(Value::as_u64),
        ) {
            (Some(path), Some(bytes)) => format!("wrote {path} ({bytes} bytes)"),
            _ => value.to_string(),
        },
        "list_files" => value
            .get("files")
            .and_then(Value::as_array)
            .map(|files| {
                if files.is_empty() {
                    "No files".into()
                } else {
                    files
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            })
            .unwrap_or_else(|| value.to_string()),
        "list_mise_tasks" => value
            .get("tasks")
            .and_then(Value::as_array)
            .map(|tasks| {
                if tasks.is_empty() {
                    "No mise tasks".into()
                } else {
                    tasks
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            })
            .unwrap_or_else(|| value.to_string()),
        "run_mise_task" => {
            let status = if value["success"] == true {
                "PASS"
            } else {
                "FAIL"
            };
            format!(
                "{status} (exit {}, truncated={}):\n{}",
                value["exit_code"],
                value["truncated"],
                value.get("output").and_then(Value::as_str).unwrap_or("")
            )
        }
        "run_task" => value
            .get("output")
            .and_then(Value::as_str)
            .filter(|output| !output.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("task success: {}", value["success"])),
        "run_flow" => value
            .get("output")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}
fn arg<'a>(value: &'a Value, key: &str) -> axllm::AxResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AxError::runtime(format!("missing string argument: {key}")))
}

// Tests -----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use std::{path::Path, sync::Arc};

    #[test]
    fn mise_tools_registered_for_chat_and_flow_nodes_without_network() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (tx, _rx) = std::sync::mpsc::channel();
        let engine = Engine::load(&root.join("agent.yaml"), &root, tx)?;
        for flow_node in [false, true] {
            let tools = engine.define_tools(flow_node, true);
            assert!(tools.iter().any(|tool| tool.name == "list_mise_tasks"));
            assert!(tools.iter().any(|tool| tool.name == "run_mise_task"));
            assert!(tools.iter().any(|tool| tool.name == "write_file"));
            assert_eq!(tools.iter().any(|tool| tool.name == "run_flow"), !flow_node);
        }
        let rendered = tool_output("list_mise_tasks", &json!({"tasks":["build", "test"]}));
        assert_eq!(rendered, "build\ntest");
        assert_eq!(
            tool_output(
                "run_mise_task",
                &json!({"output":"line 1\nline 2","success":true,"exit_code":0,"truncated":false})
            ),
            "PASS (exit 0, truncated=false):\nline 1\nline 2"
        );
        Ok(())
    }

    #[test]
    fn flow_tool_validates_arguments_before_execution() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (tx, _rx) = std::sync::mpsc::channel();
        let engine = Engine::load(&root.join("agent.yaml"), &root, tx)?;
        let flow = engine.define_tool();
        assert_eq!(flow.name, "run_flow");
        assert!(
            flow.call(json!({"name":"implement", "arguments":{"request":4}}))
                .is_err()
        );
        assert!(
            flow.call(json!({"name":"unknown", "arguments":{}}))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn read_tool_logs_multiline_text_not_escaped_json() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (tx, rx) = std::sync::mpsc::channel();
        let engine = Engine::load(&root.join("agent.yaml"), &root, tx)?;
        let tools = engine.define_tools(false, true);
        let read = tools
            .iter()
            .find(|tool| tool.name == "read_file")
            .context("read tool registered")?;
        let result = read.call(json!({"path":"agent.yaml"}))?;
        assert!(
            result["content"]
                .as_str()
                .unwrap()
                .starts_with("version: 1\n")
        );
        let output = rx
            .try_iter()
            .find_map(|event| match event {
                Event::ToolFinished { output, .. } => Some(output),
                _ => None,
            })
            .context("tool completion emitted")?;
        assert!(output.starts_with("version: 1\n"));
        assert!(!output.contains("\\n"));
        assert!(!output.contains("{\"content\""));
        Ok(())
    }

    #[test]
    fn question_tool_answers_or_skips_without_logging_the_answer() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (tx, rx) = std::sync::mpsc::channel();
        let engine = Engine::load(&root.join("agent.yaml"), &root, tx)?.with_question_handler(
            Arc::new(|_, _| QuestionAnswer::Answered {
                answer: "PRIVATE_ANSWER".into(),
            }),
        );
        let tools = engine.define_tools(false, true);
        let ask = tools
            .iter()
            .find(|tool| tool.name == "ask_user_question")
            .context("question tool is registered")?;
        let answer = ask.call(json!({"question":"Which?", "options":["Yes", "No"]}))?;
        assert_eq!(answer["status"], "answered");
        assert_eq!(answer["answer"], "PRIVATE_ANSWER");
        let output = rx
            .try_iter()
            .find_map(|event| match event {
                Event::ToolFinished { output, .. } => Some(output),
                _ => None,
            })
            .context("question completion emitted")?;
        assert_eq!(output, "question answered");
        let tools = engine.define_tools(true, false);
        let ask = tools
            .iter()
            .find(|tool| tool.name == "ask_user_question")
            .context("question tool remains available when skipped")?;
        let skipped = ask.call(json!({"question":"Which?", "options":[]}))?;
        assert_eq!(skipped["status"], "skipped");
        Ok(())
    }
}
