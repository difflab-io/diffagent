use crate::{Engine, tools::bounded};
use anyhow::{Context, Result, bail};
use diffagent_core::events::Event;
use diffagent_core::spec::{FlowNode, FlowValues, LoadedFlow, QuestionPolicy, StepValue};
use std::collections::BTreeMap;

// Types and structs -----------------------------------------------------------
/// A resolved YAML graph and its validated, per-run state. Execution still
/// dispatches each node through the appropriate Ax/host backend.
struct DefinedFlow<'a> {
    name: &'a str,
    flow: &'a LoadedFlow,
    values: FlowValues,
    current: String,
}

// Constants -------------------------------------------------------------------
const MAX_FLOW_DEPTH: usize = 1;

// Public API ------------------------------------------------------------------
impl Engine {
    pub fn run_flow(&self, name: &str, arguments: &BTreeMap<String, String>) -> Result<String> {
        let response = self.define_flow(name, arguments, 0)?;
        self.emit(Event::Finished {
            response: response.clone(),
        });
        Ok(response)
    }
}

// Helpers ---------------------------------------------------------------------
impl Engine {
    pub(crate) fn define_flow(
        &self,
        name: &str,
        arguments: &BTreeMap<String, String>,
        depth: usize,
    ) -> Result<String> {
        if depth > MAX_FLOW_DEPTH {
            bail!("flow recursion limit reached");
        }
        let defined = self.resolve_flow(name, arguments)?;
        self.execute_flow(defined)
    }

    /// Resolve a configured graph and validate its arguments before any node
    /// executes. The result retains the YAML flow's native node definitions.
    fn resolve_flow<'a>(
        &'a self,
        name: &'a str,
        arguments: &BTreeMap<String, String>,
    ) -> Result<DefinedFlow<'a>> {
        let flow = self
            .agent
            .flows
            .get(name)
            .with_context(|| format!("unknown flow: {name}"))?;
        Ok(DefinedFlow {
            name,
            current: flow.spec.start.clone(),
            values: FlowValues {
                parameters: flow.spec.arguments(arguments)?,
                ..Default::default()
            },
            flow,
        })
    }

    fn execute_flow(&self, defined: DefinedFlow<'_>) -> Result<String> {
        let DefinedFlow {
            name,
            flow,
            mut values,
            mut current,
        } = defined;
        let mut latest_output = String::new();
        for _ in 0..flow.spec.max_steps {
            if current == "end" {
                return Ok(latest_output);
            }
            let node = flow
                .spec
                .nodes
                .get(&current)
                .with_context(|| format!("unknown node: {current}"))?;
            let model = match node {
                FlowNode::Agent { model, .. } => Some(model.clone()),
                _ => None,
            };
            self.emit(Event::FlowNodeStarted {
                flow: name.into(),
                node: current.clone(),
                model,
            });
            let outcome: Result<(String, StepValue)> = (|| match node {
                FlowNode::Agent { model, next, .. } => {
                    let prompt = values.render(&flow.prompt(&current)?)?;
                    let text = self.generate(
                        model,
                        &format!("{}\n\n{}", self.instructions(), prompt),
                        true,
                        flow.spec.questions != QuestionPolicy::Skip,
                    )?;
                    Ok((
                        next.clone(),
                        StepValue {
                            output: text,
                            passed: None,
                        },
                    ))
                }
                FlowNode::Task {
                    task,
                    on_pass,
                    on_fail,
                } => {
                    let id = format!("flow:{name}:{current}:task");
                    self.emit(Event::ToolStarted {
                        id: id.clone(),
                        name: "run_task".into(),
                        detail: task.clone(),
                    });
                    let result = self.host.run_named_task(task);
                    self.emit(Event::ToolFinished {
                        id,
                        success: result.as_ref().is_ok_and(|output| output.success),
                        output: match &result {
                            Ok(output) => bounded(&output.output),
                            Err(error) => error.to_string(),
                        },
                    });
                    let output = result?;
                    let next = task_target(output.success, on_pass, on_fail);
                    Ok((
                        next.into(),
                        StepValue {
                            output: output.output,
                            passed: Some(output.success),
                        },
                    ))
                }
                FlowNode::End => Ok(("end".into(), StepValue::default())),
            })();
            self.emit(Event::FlowNodeFinished {
                flow: name.into(),
                node: current.clone(),
                success: outcome
                    .as_ref()
                    .is_ok_and(|(_, step)| step.passed.unwrap_or(true)),
            });
            let (next, step) = outcome?;
            latest_output = step.output.clone();
            values.steps.insert(current, step);
            current = next;
        }
        if current == "end" {
            return Ok(latest_output);
        }
        bail!("flow {name} exceeded max_steps ({})", flow.spec.max_steps)
    }
}

fn task_target<'a>(passed: bool, on_pass: &'a str, on_fail: &'a str) -> &'a str {
    if passed { on_pass } else { on_fail }
}

// Tests -----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use diffagent_core::spec::LoadedAgent;
    use std::path::Path;

    #[test]
    fn config_and_branching_without_network() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agent = LoadedAgent::load(&root.join("agent.yaml")).unwrap();
        let flow = &agent.flows["implement"].spec;
        let args = flow
            .arguments(&BTreeMap::from([("request".into(), "hello".into())]))
            .unwrap();
        assert_eq!(args["request"], "hello");
        assert!(flow.arguments(&BTreeMap::new()).is_err());
        match &flow.nodes["evaluate"] {
            FlowNode::Task {
                on_pass, on_fail, ..
            } => {
                assert_eq!(on_pass, "end");
                assert_eq!(on_fail, "update");
            }
            _ => panic!("expected task node"),
        }
        assert_eq!(flow.max_steps, 12);
        assert_eq!(task_target(true, "end", "update"), "end");
        assert_eq!(task_target(false, "end", "update"), "update");
    }

    #[test]
    fn explicit_flow_rejects_missing_args_and_recursion_without_network() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (tx, _rx) = std::sync::mpsc::channel();
        let engine = Engine::load(&root.join("agent.yaml"), &root, tx).unwrap();
        assert!(
            engine
                .define_flow("implement", &BTreeMap::new(), 0)
                .unwrap_err()
                .to_string()
                .contains("missing flow argument")
        );
        assert!(
            engine
                .define_flow("implement", &BTreeMap::new(), MAX_FLOW_DEPTH + 1)
                .unwrap_err()
                .to_string()
                .contains("recursion limit")
        );
        assert!(
            engine
                .define_flow("unknown", &BTreeMap::new(), 0)
                .unwrap_err()
                .to_string()
                .contains("unknown flow")
        );
    }
}
