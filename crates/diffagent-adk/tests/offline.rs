use diffagent_adk::execute_flow;
use diffagent_core::{
    events::Event,
    spec::{FlowNode, LoadedAgent},
    tools::ToolHost,
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, mpsc},
};

// Helpers ---------------------------------------------------------------------

fn fixture() -> LoadedAgent {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agent.yaml");
    LoadedAgent::load(&path).unwrap()
}

// Tests -----------------------------------------------------------------------

#[test]
fn loads_configuration_and_skill_instructions() {
    let agent = fixture();
    assert_eq!(agent.spec.model, "deepseek-flash");
    assert!(
        agent
            .skills
            .iter()
            .any(|skill| !skill.instructions.is_empty())
    );
    let flow = &agent.flows["implement"];
    assert!(
        flow.prompt("implement")
            .unwrap()
            .contains("steps.plan.output")
    );
    assert!(flow.spec.arguments(&BTreeMap::new()).is_err());
    assert!(
        flow.spec
            .arguments(&BTreeMap::from([("request".into(), "hi".into())]))
            .is_ok()
    );
}

#[tokio::test]
async fn end_node_runs_without_model_or_network() {
    let mut agent = fixture();
    let flow = agent.flows.get_mut("implement").unwrap();
    flow.spec.nodes.insert("plan".into(), FlowNode::End);
    let host = ToolHost::new(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        agent.spec.tools.clone(),
    )
    .unwrap();
    let (tx, rx) = mpsc::channel();
    let result = execute_flow(
        &Arc::new(agent),
        host,
        "implement",
        &BTreeMap::from([("request".into(), "hello".into())]),
        &tx,
    )
    .await
    .unwrap();
    assert_eq!(result, "");
    assert!(rx.try_iter().any(|event| matches!(event, Event::FlowNodeFinished { node, success: true, .. } if node == "plan")));
}

#[tokio::test]
async fn rejects_unknown_flow_and_arguments_without_network() {
    let agent = fixture();
    let host = ToolHost::new(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        agent.spec.tools.clone(),
    )
    .unwrap();
    let (tx, _) = mpsc::channel();
    let agent = Arc::new(agent);
    assert!(
        execute_flow(&agent, host.clone(), "missing", &BTreeMap::new(), &tx)
            .await
            .is_err()
    );
    assert!(
        execute_flow(&agent, host, "implement", &BTreeMap::new(), &tx)
            .await
            .is_err()
    );
}
