use std::{collections::BTreeMap, future::Future, sync::mpsc::Sender};

use anyhow::{Result, anyhow, bail};
use diffagent_core::{
    events::Event,
    spec::{FlowNode, FlowValues, LoadedFlow, QuestionPolicy, StepValue},
    tools::ToolHost,
};

use crate::agent::{FLOW_TURNS, MAX_FLOW_DEPTH, RigRuntime, call_id, send};

// Public API ------------------------------------------------------------------
impl RigRuntime {
    /// Run a declared flow explicitly, without entering chat or changing chat history.
    pub async fn flow(&self, name: &str, arguments: BTreeMap<String, String>) -> Result<String> {
        let output = self.define_flow(name, arguments, 0).await?;
        send(
            &self.events,
            Event::Finished {
                response: output.clone(),
            },
        );
        Ok(output)
    }
}

/// Execute a validated graph. The injected agent callback makes control-flow and
/// task branches testable without a provider or network access.
pub async fn execute_flow<F, Fut>(
    flow: &LoadedFlow,
    supplied: &BTreeMap<String, String>,
    host: &ToolHost,
    events: &Sender<Event>,
    mut agent_step: F,
) -> Result<String>
where
    F: FnMut(String, String) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let mut values = FlowValues {
        parameters: flow.spec.arguments(supplied)?,
        ..Default::default()
    };
    let mut node_name = flow.spec.start.clone();
    let mut last = String::new();
    for _ in 0..flow.spec.max_steps {
        if node_name == "end" {
            return Ok(last);
        }
        let node = flow
            .spec
            .nodes
            .get(&node_name)
            .ok_or_else(|| anyhow!("unknown node: {node_name}"))?;
        let model = match node {
            FlowNode::Agent { model, .. } => Some(model.clone()),
            _ => None,
        };
        send(
            events,
            Event::FlowNodeStarted {
                flow: flow.spec.name.clone(),
                node: node_name.clone(),
                model,
            },
        );
        let outcome: Result<(String, Option<bool>, String)> = match node {
            FlowNode::Agent { model, next, .. } => {
                match flow
                    .prompt(&node_name)
                    .and_then(|template| values.render(&template))
                {
                    Ok(prompt) => agent_step(model.clone(), prompt)
                        .await
                        .map(|output| (output, None, next.clone())),
                    Err(error) => Err(error.into()),
                }
            }
            FlowNode::Task {
                task,
                on_pass,
                on_fail,
            } => {
                let id = call_id();
                send(
                    events,
                    Event::ToolStarted {
                        id: id.clone(),
                        name: "run_task".into(),
                        detail: task.clone(),
                    },
                );
                let result = host.run_named_task(task);
                let (passed, output) = match result {
                    Ok(result) => (result.success, result.output),
                    Err(error) => (false, error.to_string()),
                };
                send(
                    events,
                    Event::ToolFinished {
                        id,
                        success: passed,
                        output: output.clone(),
                    },
                );
                Ok((
                    output,
                    Some(passed),
                    if passed { on_pass } else { on_fail }.clone(),
                ))
            }
            FlowNode::End => Ok((last.clone(), None, "end".into())),
        };
        send(
            events,
            Event::FlowNodeFinished {
                flow: flow.spec.name.clone(),
                node: node_name.clone(),
                success: outcome.is_ok(),
            },
        );
        let (output, passed, next) = outcome?;
        last = output.clone();
        values.steps.insert(node_name, StepValue { output, passed });
        node_name = next;
    }
    if node_name == "end" {
        Ok(last)
    } else {
        bail!(
            "flow {} exceeded max_steps ({})",
            flow.spec.name,
            flow.spec.max_steps
        )
    }
}

// Helpers ---------------------------------------------------------------------
impl RigRuntime {
    /// Adapt a declared YAML flow to Rig model steps at the requested nesting depth.
    pub(crate) async fn define_flow(
        &self,
        name: &str,
        arguments: BTreeMap<String, String>,
        depth: usize,
    ) -> Result<String> {
        if depth >= MAX_FLOW_DEPTH {
            bail!("run_flow recursion limit exceeded");
        }
        let flow = self
            .agent
            .flows
            .get(name)
            .ok_or_else(|| anyhow!("unknown flow: {name}"))?;
        let allow_questions = flow.spec.questions != QuestionPolicy::Skip;
        execute_flow(
            flow,
            &arguments,
            &self.host,
            &self.events,
            |model, prompt| {
                let runtime = self.clone();
                async move {
                    Ok(runtime
                        .model_run(
                            &model,
                            &prompt,
                            None,
                            depth + 1,
                            FLOW_TURNS,
                            allow_questions,
                        )
                        .await?
                        .output)
                }
            },
        )
        .await
    }
}

// Tests -----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use diffagent_core::spec::{FlowSpec, ToolSpec};
    use std::sync::{Arc, Mutex, mpsc};

    fn fixture(
        yaml: &str,
    ) -> (
        tempfile::TempDir,
        LoadedFlow,
        ToolHost,
        Sender<Event>,
        mpsc::Receiver<Event>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let spec: FlowSpec = serde_yaml::from_str(yaml).unwrap();
        spec.validate().unwrap();
        let host = ToolHost::new(
            dir.path(),
            ToolSpec {
                max_write_bytes: 1024,
                ..Default::default()
            },
        )
        .unwrap();
        let (tx, rx) = mpsc::channel();
        let flow = LoadedFlow {
            spec,
            root: dir.path().into(),
        };
        (dir, flow, host, tx, rx)
    }

    #[tokio::test]
    async fn renders_params_and_prior_step_output_and_uses_node_model() {
        let (dir, flow, host, tx, rx) = fixture(
            "version: 1\nname: example\nstart: plan\nparameters:\n  request: {type: string, required: true}\nnodes:\n  plan: {kind: agent, model: first, prompt: plan.md, next: implement}\n  implement: {kind: agent, model: second, prompt: implement.md, next: end}\n",
        );
        std::fs::write(dir.path().join("plan.md"), "Plan {{ params.request }}").unwrap();
        std::fs::write(
            dir.path().join("implement.md"),
            "Do {{ steps.plan.output }}",
        )
        .unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let capture = seen.clone();
        let output = execute_flow(
            &flow,
            &BTreeMap::from([("request".into(), "game".into())]),
            &host,
            &tx,
            move |model, prompt| {
                capture.lock().unwrap().push((model, prompt.clone()));
                async move { Ok(prompt) }
            },
        )
        .await
        .unwrap();
        assert_eq!(output, "Do Plan game");
        assert_eq!(
            *seen.lock().unwrap(),
            [
                ("first".into(), "Plan game".into()),
                ("second".into(), "Do Plan game".into())
            ]
        );
        assert_eq!(
            rx.try_iter()
                .filter(|e| matches!(e, Event::FlowNodeFinished { success: true, .. }))
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn rejects_missing_and_unknown_parameters_before_invocation() {
        let (dir, flow, host, tx, _) = fixture(
            "version: 1\nname: example\nstart: one\nparameters:\n  request: {type: string, required: true}\nnodes:\n  one: {kind: agent, model: m, prompt: one.md, next: end}\n",
        );
        std::fs::write(dir.path().join("one.md"), "hello").unwrap();
        let call = |_: String, _: String| async {
            panic!("model must not be called");
            #[allow(unreachable_code)]
            Ok(String::new())
        };
        assert!(
            execute_flow(&flow, &BTreeMap::new(), &host, &tx, call)
                .await
                .is_err()
        );
        assert!(
            execute_flow(
                &flow,
                &BTreeMap::from([("unknown".into(), "x".into())]),
                &host,
                &tx,
                call
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn stops_cycles_at_max_steps_without_api_calls() {
        let (dir, flow, host, tx, _) = fixture(
            "version: 1\nname: loop\nstart: one\nmax_steps: 2\nnodes:\n  one: {kind: agent, model: m, prompt: one.md, next: one}\n",
        );
        std::fs::write(dir.path().join("one.md"), "again").unwrap();
        let error = execute_flow(&flow, &BTreeMap::new(), &host, &tx, |_, _| async {
            Ok("done".into())
        })
        .await
        .unwrap_err();
        assert!(error.to_string().contains("max_steps"));
    }

    #[tokio::test]
    async fn failed_task_branches_and_exposes_pass_result() {
        let (dir, flow, host, tx, rx) = fixture(
            "version: 1\nname: branch\nstart: check\nnodes:\n  check: {kind: task, task: missing, on_pass: end, on_fail: explain}\n  explain: {kind: agent, model: m, prompt: explain.md, next: end}\n",
        );
        std::fs::write(
            dir.path().join("explain.md"),
            "passed={{ steps.check.passed }}",
        )
        .unwrap();
        let result = execute_flow(
            &flow,
            &BTreeMap::new(),
            &host,
            &tx,
            |_, prompt| async move { Ok(prompt) },
        )
        .await
        .unwrap();
        assert_eq!(result, "passed=false");
        assert!(
            rx.try_iter()
                .any(|event| matches!(event, Event::ToolFinished { success: false, .. }))
        );
    }
}
