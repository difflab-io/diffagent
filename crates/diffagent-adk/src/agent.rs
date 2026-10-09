use adk_agent::LlmAgentBuilder;
use adk_core::{Agent, Llm};
use adk_model::deepseek::{DeepSeekClient, DeepSeekConfig};
use anyhow::{Context, Result};
use diffagent_core::{
    events::{Event, QuestionAnswer, QuestionHandler},
    spec::LoadedAgent,
    tools::ToolHost,
};
use std::sync::{Arc, mpsc::Sender};

// Public API ------------------------------------------------------------------

pub(crate) fn define_agent(
    agent: &Arc<LoadedAgent>,
    host: ToolHost,
    llm: Arc<dyn Llm>,
    tx: &Sender<Event>,
    allow_flow: bool,
    question_handler: QuestionHandler,
    allow_questions: bool,
) -> Result<Arc<dyn Agent>> {
    let mut builder = LlmAgentBuilder::new("coding")
        .model(llm)
        .instruction(instruction(agent))
        .max_iterations(10);
    for tool in crate::tools::define_tools(
        agent,
        host,
        tx,
        allow_flow,
        question_handler,
        allow_questions,
    ) {
        builder = builder.tool(tool);
    }
    Ok(Arc::new(builder.build()?))
}

// Helpers ---------------------------------------------------------------------

pub(crate) fn default_question_handler() -> QuestionHandler {
    Arc::new(|_, _| QuestionAnswer::Skipped {
        reason: "question UI unavailable".into(),
    })
}

pub(crate) fn model(name: &str) -> Result<Arc<dyn Llm>> {
    let key = std::env::var("DEEPSEEK_API_KEY")
        .context("DEEPSEEK_API_KEY required for ADK DeepSeek model")?;
    Ok(Arc::new(DeepSeekClient::new(DeepSeekConfig::new(
        key, name,
    ))?))
}

pub(crate) fn instruction(agent: &LoadedAgent) -> String {
    let mut text = agent.system_prompt.clone();
    for skill in &agent.skills {
        text.push_str("\n\n# Skill: ");
        text.push_str(&skill.name);
        text.push('\n');
        text.push_str(&skill.instructions);
    }
    text
}
