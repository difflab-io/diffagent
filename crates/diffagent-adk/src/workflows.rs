use adk_core::{Content, SessionId, UserId};
use adk_runner::Runner;
use adk_session::{CreateRequest, InMemorySessionService, SessionService};
use anyhow::{Context, Result, bail};
use diffagent_core::{
    events::{Event, QuestionHandler},
    spec::{FlowNode, FlowValues, LoadedAgent, QuestionPolicy, StepValue},
    tools::ToolHost,
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, mpsc::Sender},
};

use crate::{
    agent::{default_question_handler, define_agent, model},
    chat::drain,
    tools::emit,
};

// Public API ------------------------------------------------------------------

/// Run a selected graph only when explicitly requested. Flow agent nodes cannot call flows.
pub async fn execute_flow(
    agent: &Arc<LoadedAgent>,
    host: ToolHost,
    name: &str,
    args: &BTreeMap<String, String>,
    tx: &Sender<Event>,
) -> Result<String> {
    execute_flow_with_question_handler(agent, host, name, args, tx, default_question_handler())
        .await
}

pub async fn execute_flow_with_question_handler(
    agent: &Arc<LoadedAgent>,
    host: ToolHost,
    name: &str,
    args: &BTreeMap<String, String>,
    tx: &Sender<Event>,
    handler: QuestionHandler,
) -> Result<String> {
    define_flow(agent, host, name, args, tx, handler).await
}

pub async fn run_flow(
    agent: LoadedAgent,
    workspace: &Path,
    name: &str,
    args: &BTreeMap<String, String>,
    tx: &Sender<Event>,
) -> Result<String> {
    run_flow_with_question_handler(agent, workspace, name, args, tx, default_question_handler())
        .await
}

pub async fn run_flow_with_question_handler(
    agent: LoadedAgent,
    workspace: &Path,
    name: &str,
    args: &BTreeMap<String, String>,
    tx: &Sender<Event>,
    handler: QuestionHandler,
) -> Result<String> {
    let host = ToolHost::new(workspace, agent.spec.tools.clone())?;
    let response =
        execute_flow_with_question_handler(&Arc::new(agent), host, name, args, tx, handler).await?;
    emit(
        tx,
        Event::Finished {
            response: response.clone(),
        },
    );
    Ok(response)
}

// Helpers ---------------------------------------------------------------------

/// Execute the selected ADK flow graph, including node runners, tasks, and events.
pub(crate) async fn define_flow(
    agent: &Arc<LoadedAgent>,
    host: ToolHost,
    name: &str,
    args: &BTreeMap<String, String>,
    tx: &Sender<Event>,
    handler: QuestionHandler,
) -> Result<String> {
    let flow = agent
        .flows
        .get(name)
        .with_context(|| format!("unknown flow: {name}"))?;
    let mut values = FlowValues {
        parameters: flow.spec.arguments(args)?,
        ..Default::default()
    };
    let mut node = flow.spec.start.clone();
    let mut last = String::new();
    for _ in 0..flow.spec.max_steps {
        if node == "end" {
            return Ok(last);
        }
        let current = flow
            .spec
            .nodes
            .get(&node)
            .with_context(|| format!("missing node: {node}"))?;
        let selected_model = match current {
            FlowNode::Agent { model, .. } => Some(model.clone()),
            _ => None,
        };
        emit(
            tx,
            Event::FlowNodeStarted {
                flow: name.into(),
                node: node.clone(),
                model: selected_model,
            },
        );
        if matches!(current, FlowNode::End) {
            emit(
                tx,
                Event::FlowNodeFinished {
                    flow: name.into(),
                    node,
                    success: true,
                },
            );
            return Ok(last);
        }
        let outcome: Result<(String, StepValue)> = async {
            Ok(match current {
                FlowNode::End => unreachable!(),
                FlowNode::Agent {
                    model: model_name,
                    next,
                    ..
                } => {
                    let prompt = values.render(&flow.prompt(&node)?)?;
                    let root = define_agent(
                        agent,
                        host.clone(),
                        model(model_name)?,
                        tx,
                        false,
                        handler.clone(),
                        flow.spec.questions != QuestionPolicy::Skip,
                    )?;
                    let sessions =
                        Arc::new(InMemorySessionService::new()) as Arc<dyn SessionService>;
                    let session_id = SessionId::new("node")?;
                    sessions
                        .create(CreateRequest {
                            app_name: "diffagent-flow".into(),
                            user_id: "local-user".into(),
                            session_id: Some("node".into()),
                            state: Default::default(),
                        })
                        .await?;
                    let runner = Runner::builder()
                        .app_name("diffagent-flow")
                        .agent(root)
                        .session_service(sessions)
                        .build()?;
                    let stream = runner
                        .run(
                            UserId::new("local-user")?,
                            session_id,
                            Content::new("user").with_text(&prompt),
                        )
                        .await?;
                    let output = drain(stream, tx, "coding", false).await?;
                    (
                        next.clone(),
                        StepValue {
                            output,
                            passed: None,
                        },
                    )
                }
                FlowNode::Task {
                    task,
                    on_pass,
                    on_fail,
                } => {
                    let task_name = task.clone();
                    let task_host = host.clone();
                    let id = format!("flow:{name}:{node}:task");
                    emit(
                        tx,
                        Event::ToolStarted {
                            id: id.clone(),
                            name: "run_task".into(),
                            detail: task_name.clone(),
                        },
                    );
                    let result =
                        tokio::task::spawn_blocking(move || task_host.run_named_task(&task_name))
                            .await?;
                    emit(
                        tx,
                        Event::ToolFinished {
                            id,
                            success: result.as_ref().is_ok_and(|output| output.success),
                            output: result
                                .as_ref()
                                .map(|output| output.output.clone())
                                .unwrap_or_else(|error| error.to_string()),
                        },
                    );
                    let output = result?;
                    let next = if output.success { on_pass } else { on_fail };
                    (
                        next.clone(),
                        StepValue {
                            output: output.output,
                            passed: Some(output.success),
                        },
                    )
                }
            })
        }
        .await;
        let (next, step) = match outcome {
            Ok(value) => value,
            Err(error) => {
                emit(
                    tx,
                    Event::FlowNodeFinished {
                        flow: name.into(),
                        node,
                        success: false,
                    },
                );
                return Err(error);
            }
        };
        last = step.output.clone();
        values.steps.insert(node.clone(), step);
        emit(
            tx,
            Event::FlowNodeFinished {
                flow: name.into(),
                node,
                success: true,
            },
        );
        node = next;
    }
    bail!("flow {name} exceeded max_steps ({})", flow.spec.max_steps)
}
