use std::collections::BTreeMap;

use diffagent_core::{
    events::{Event, QuestionAnswer},
    tools::ToolError,
};
use rig::{
    agent::{AgentBuilder, NoToolConfig, WithBuilderTools},
    tool::{Tool, ToolContext},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::agent::{RigRuntime, call_id, send};

// Types and structs -----------------------------------------------------------
#[derive(Deserialize)]
pub(crate) struct PathArg {
    path: String,
}
#[derive(Deserialize)]
pub(crate) struct WriteArg {
    path: String,
    content: String,
}
#[derive(Deserialize)]
pub(crate) struct TaskArg {
    task: String,
}
#[derive(Deserialize)]
pub(crate) struct MiseTaskArg {
    name: String,
}
#[derive(Deserialize)]
pub(crate) struct FlowArg {
    name: String,
    #[serde(default)]
    arguments: BTreeMap<String, String>,
}

#[derive(Deserialize)]
pub(crate) struct QuestionArg {
    question: String,
    options: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct QuestionTool {
    pub(crate) runtime: RigRuntime,
    pub(crate) allow_questions: bool,
}
#[derive(Clone)]
pub(crate) struct ReadTool(pub(crate) RigRuntime);
#[derive(Clone)]
pub(crate) struct WriteTool(pub(crate) RigRuntime);
#[derive(Clone)]
pub(crate) struct ListTool(pub(crate) RigRuntime);
#[derive(Clone)]
pub(crate) struct TaskTool(pub(crate) RigRuntime);
#[derive(Clone)]
pub(crate) struct ListMiseTasksTool(pub(crate) RigRuntime);
#[derive(Clone)]
pub(crate) struct RunMiseTaskTool(pub(crate) RigRuntime);
#[derive(Clone)]
pub(crate) struct FlowTool {
    pub(crate) runtime: RigRuntime,
    pub(crate) depth: usize,
}

// Public API ------------------------------------------------------------------
impl Tool for QuestionTool {
    const NAME: &'static str = "ask_user_question";
    type Args = QuestionArg;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        "Ask one question. Pass options: [] for a free-text answer, or one to four choices. A skipped response is not an answer.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"question":{"type":"string","maxLength":4096},"options":{"type":"array","items":{"type":"string"},"maxItems":4}},"required":["question","options"]})
    }
    async fn call(
        &self,
        _: &mut ToolContext,
        args: QuestionArg,
    ) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.runtime.started(&id, Self::NAME, &args.question);
        let answer = if args.question.trim().is_empty()
            || args.question.len() > 4096
            || args.options.len() > 4
            || args
                .options
                .iter()
                .any(|option| option.trim().is_empty() || option.len() > 256)
        {
            Err(ToolError::Denied(
                "question must be nonempty, at most 4096 bytes, with at most four nonempty options of 256 bytes each".into(),
            ))
        } else if self.allow_questions {
            let handler = self.runtime.question_handler.clone();
            tokio::task::spawn_blocking(move || handler(&args.question, &args.options))
                .await
                .map_err(|e| ToolError::Denied(e.to_string()))
        } else {
            Ok(QuestionAnswer::Skipped {
                reason: "questions disabled by flow policy".into(),
            })
        };
        let summary = answer.as_ref().ok().map(|value| value.summary().to_owned());
        let result = answer.and_then(|value| {
            serde_json::to_string(&value).map_err(|error| ToolError::Denied(error.to_string()))
        });
        send(
            &self.runtime.events,
            Event::ToolFinished {
                id,
                success: result.is_ok(),
                output: match (&summary, &result) {
                    (Some(summary), Ok(_)) => summary.clone(),
                    (_, Err(error)) => error.to_string(),
                    _ => String::new(),
                },
            },
        );
        result
    }
}

impl Tool for ReadTool {
    const NAME: &'static str = "read_file";
    type Args = PathArg;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        "Read an allowlisted workspace file".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    async fn call(
        &self,
        _: &mut ToolContext,
        args: PathArg,
    ) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.0.started(&id, Self::NAME, &args.path);
        let result = self.0.host.read_file(&args.path);
        self.0.finished(&id, &result);
        result
    }
}

impl Tool for WriteTool {
    const NAME: &'static str = "write_file";
    type Args = WriteArg;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        "Propose complete content for an allowlisted workspace file, then commit atomically".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    }
    async fn call(
        &self,
        _: &mut ToolContext,
        args: WriteArg,
    ) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.0.started(&id, Self::NAME, &args.path);
        let events = self.0.events.clone();
        let result = self
            .0
            .host
            .write_file(&args.path, &args.content, |proposal| {
                let content = std::str::from_utf8(proposal.content)
                    .map_err(|e| ToolError::Preview(e.to_string()))?;
                send(
                    &events,
                    Event::WritePreview {
                        id: id.clone(),
                        path: proposal.relative.into(),
                        original: proposal.original.map(str::to_owned),
                        content: content.into(),
                    },
                );
                Ok(())
            })
            .map(|receipt| {
                send(
                    &self.0.events,
                    Event::WriteCommitted {
                        id: id.clone(),
                        path: receipt.relative.clone(),
                    },
                );
                format!("wrote {} ({} bytes)", receipt.relative, receipt.bytes)
            });
        self.0.finished(&id, &result);
        result
    }
}

impl Tool for ListTool {
    const NAME: &'static str = "list_files";
    type Args = serde_json::Value;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        "List allowlisted readable files in the workspace".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    async fn call(&self, _: &mut ToolContext, _: Value) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.0.started(&id, Self::NAME, "workspace");
        let result = self.0.host.list_files().map(|files| files.join("\n"));
        self.0.finished(&id, &result);
        result
    }
}

impl Tool for TaskTool {
    const NAME: &'static str = "run_task";
    type Args = TaskArg;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        format!(
            "Run a configured named task in the sandbox; returns status and output. Available tasks: {}",
            self.0
                .agent
                .spec
                .tools
                .tasks
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"task":{"type":"string"}},"required":["task"]})
    }
    async fn call(
        &self,
        _: &mut ToolContext,
        args: TaskArg,
    ) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.0.started(&id, Self::NAME, &args.task);
        let host = self.0.host.clone();
        let outcome = tokio::task::spawn_blocking(move || host.run_named_task(&args.task))
            .await
            .map_err(|error| ToolError::Denied(error.to_string()))?;
        let passed = outcome.as_ref().is_ok_and(|output| output.success);
        let result = outcome.map(|output| {
            format!(
                "{} (exit {:?}, truncated={}):\n{}",
                if output.success { "PASS" } else { "FAIL" },
                output.exit_code,
                output.truncated,
                output.output
            )
        });
        send(
            &self.0.events,
            Event::ToolFinished {
                id,
                success: passed,
                output: result.as_ref().map_or_else(|e| e.to_string(), Clone::clone),
            },
        );
        result
    }
}

impl Tool for ListMiseTasksTool {
    const NAME: &'static str = "list_mise_tasks";
    type Args = Value;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        "List all mise tasks visible in this workspace, including tasks defined in mise.toml".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    async fn call(&self, _: &mut ToolContext, _: Value) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.0.started(&id, Self::NAME, "workspace");
        let result = self.0.host.list_mise_tasks();
        send(
            &self.0.events,
            Event::ToolFinished {
                id,
                success: result.is_ok(),
                output: result.as_ref().map_or_else(
                    |error| error.to_string(),
                    |tasks| {
                        bounded_event(&if tasks.is_empty() {
                            "No mise tasks".into()
                        } else {
                            tasks.join("\n")
                        })
                    },
                ),
            },
        );
        result.map(|tasks| json!({"tasks": tasks}).to_string())
    }
}

impl Tool for RunMiseTaskTool {
    const NAME: &'static str = "run_mise_task";
    type Args = MiseTaskArg;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        "Run a mise task visible in this workspace by name. Use list_mise_tasks to discover names; no arbitrary shell commands.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]})
    }
    async fn call(
        &self,
        _: &mut ToolContext,
        args: MiseTaskArg,
    ) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.0.started(&id, Self::NAME, &args.name);
        let host = self.0.host.clone();
        let result = tokio::task::spawn_blocking(move || host.run_mise_task(&args.name))
            .await
            .map_err(|error| ToolError::Denied(error.to_string()))
            .and_then(|result| result);
        send(
            &self.0.events,
            Event::ToolFinished {
                id,
                success: result.as_ref().is_ok_and(|output| output.success),
                output: result.as_ref().map_or_else(
                    |error| error.to_string(),
                    |output| {
                        bounded_event(&format!(
                            "{} (exit {:?}, truncated={}):\n{}",
                            if output.success { "PASS" } else { "FAIL" },
                            output.exit_code,
                            output.truncated,
                            output.output
                        ))
                    },
                ),
            },
        );
        result.map(|output| {
            json!({
                "success":output.success,"exit_code":output.exit_code,
                "output":output.output,"truncated":output.truncated
            })
            .to_string()
        })
    }
}

impl Tool for FlowTool {
    const NAME: &'static str = "run_flow";
    type Args = FlowArg;
    type Output = String;
    type Error = ToolError;
    fn description(&self) -> String {
        format!(
            "Explicitly run a configured YAML flow. Available flows: {}. Supply its named string arguments.",
            self.runtime
                .agent
                .flows
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"name":{"type":"string"},"arguments":{"type":"object","additionalProperties":{"type":"string"}}},"required":["name"]})
    }
    async fn call(
        &self,
        _: &mut ToolContext,
        args: FlowArg,
    ) -> std::result::Result<String, ToolError> {
        let id = call_id();
        self.runtime.started(&id, Self::NAME, &args.name);
        let result = self
            .runtime
            .define_flow(&args.name, args.arguments, self.depth)
            .await
            .map_err(|e| ToolError::Denied(e.to_string()));
        send(
            &self.runtime.events,
            Event::ToolFinished {
                id,
                success: result.is_ok(),
                output: result.as_ref().map_or_else(|e| e.to_string(), Clone::clone),
            },
        );
        result
    }
}

/// Register Rig's typed `Tool` implementations without erasing their types.
/// The first registration transitions the builder from NoToolConfig to WithBuilderTools.
pub(crate) fn define_tools(
    builder: AgentBuilder<NoToolConfig>,
    runtime: &RigRuntime,
    depth: usize,
    allow_questions: bool,
) -> AgentBuilder<WithBuilderTools> {
    let mut builder = builder
        .tool(ReadTool(runtime.clone()))
        .tool(WriteTool(runtime.clone()))
        .tool(ListTool(runtime.clone()))
        .tool(TaskTool(runtime.clone()));
    if runtime.agent.spec.tools.mise_tasks {
        builder = builder
            .tool(ListMiseTasksTool(runtime.clone()))
            .tool(RunMiseTaskTool(runtime.clone()));
    }
    builder
        .tool(QuestionTool {
            runtime: runtime.clone(),
            allow_questions,
        })
        .tool(define_tool(runtime, depth))
}

// Helpers ---------------------------------------------------------------------
/// Build the Rig-native tool that lets chat explicitly run a YAML flow.
fn define_tool(runtime: &RigRuntime, depth: usize) -> FlowTool {
    FlowTool {
        runtime: runtime.clone(),
        depth,
    }
}

fn bounded_event(text: &str) -> String {
    text.chars().take(12_000).collect()
}

// Tests -----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use diffagent_core::spec::LoadedAgent;
    use std::sync::{Arc, mpsc};

    #[test]
    fn mise_tool_schemas_and_bounded_event_output() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agent = LoadedAgent::load(&root.join("agent.yaml")).unwrap();
        let host = diffagent_core::tools::ToolHost::new(&root, agent.spec.tools.clone()).unwrap();
        let (tx, _rx) = mpsc::channel();
        let runtime = RigRuntime::new(agent, host, tx);
        assert_eq!(
            ListMiseTasksTool(runtime.clone()).parameters(),
            json!({"type":"object","properties":{}})
        );
        assert_eq!(
            RunMiseTaskTool(runtime).parameters()["required"],
            json!(["name"])
        );
        assert_eq!(bounded_event(&"x".repeat(20_000)).len(), 12_000);
    }

    #[tokio::test]
    async fn question_tool_schema_skip_policy_and_unavailable_ui() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agent = LoadedAgent::load(&root.join("agent.yaml")).unwrap();
        let host = diffagent_core::tools::ToolHost::new(&root, agent.spec.tools.clone()).unwrap();
        let (tx, rx) = mpsc::channel();
        let runtime = RigRuntime::new(agent, host, tx);
        let tool = QuestionTool {
            runtime,
            allow_questions: false,
        };
        assert_eq!(
            tool.parameters()["required"],
            json!(["question", "options"])
        );
        let mut context = ToolContext::default();
        let output = tool
            .call(
                &mut context,
                QuestionArg {
                    question: "Proceed?".into(),
                    options: vec!["Yes".into()],
                },
            )
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&output).unwrap()["status"],
            "skipped"
        );
        assert!(output.contains("flow policy"));
        assert!(
            rx.try_iter()
                .any(|event| matches!(event, Event::ToolFinished { success: true, .. }))
        );
        let tool = QuestionTool {
            allow_questions: true,
            ..tool
        };
        let output = tool
            .call(
                &mut context,
                QuestionArg {
                    question: "Proceed?".into(),
                    options: vec![],
                },
            )
            .await
            .unwrap();
        assert!(output.contains("question UI unavailable"));
    }

    #[tokio::test]
    async fn answered_question_does_not_log_the_private_answer() -> Result<()> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agent = LoadedAgent::load(&root.join("agent.yaml"))?;
        let host = diffagent_core::tools::ToolHost::new(&root, agent.spec.tools.clone())?;
        let (tx, rx) = mpsc::channel();
        let runtime = RigRuntime::new(agent, host, tx).with_question_handler(Arc::new(|_, _| {
            QuestionAnswer::Answered {
                answer: "PRIVATE_ANSWER".into(),
            }
        }));
        let tool = QuestionTool {
            runtime,
            allow_questions: true,
        };
        let answer = tool
            .call(
                &mut ToolContext::default(),
                QuestionArg {
                    question: "Which?".into(),
                    options: vec![],
                },
            )
            .await?;
        assert!(answer.contains("PRIVATE_ANSWER"));
        let outputs: Vec<String> = rx
            .try_iter()
            .filter_map(|event| match event {
                Event::ToolFinished { output, .. } => Some(output),
                _ => None,
            })
            .collect();
        assert_eq!(outputs, ["question answered"]);
        Ok(())
    }
}
