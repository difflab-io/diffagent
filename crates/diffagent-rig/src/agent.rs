use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::Sender,
    },
};

use anyhow::{Result, bail};
use diffagent_core::{
    events::{ActivityGuard, Event, QuestionAnswer, QuestionHandler},
    spec::LoadedAgent,
    tools::{ToolError, ToolHost},
};
use futures::StreamExt;
use rig::{
    agent::{Agent, AgentBuilder, MultiTurnStreamItem, PromptResponse},
    completion::Message,
    streaming::{Item, StreamEvent},
};

use crate::tools::define_tools;

// Types and structs -----------------------------------------------------------
#[derive(Clone)]
pub struct RigRuntime {
    pub(crate) agent: Arc<LoadedAgent>,
    pub(crate) host: ToolHost,
    pub(crate) events: Sender<Event>,
    pub(crate) question_handler: QuestionHandler,
}

// Public API ------------------------------------------------------------------
impl RigRuntime {
    pub fn new(agent: LoadedAgent, host: ToolHost, events: Sender<Event>) -> Self {
        Self {
            agent: Arc::new(agent),
            host,
            events,
            question_handler: Arc::new(|_, _| QuestionAnswer::Skipped {
                reason: "question UI unavailable".into(),
            }),
        }
    }

    pub fn with_question_handler(mut self, handler: QuestionHandler) -> Self {
        self.question_handler = handler;
        self
    }

    pub fn emit_error(&self, message: String) {
        send(&self.events, Event::Error { message });
    }
}

// Helpers ---------------------------------------------------------------------
pub(crate) const CHAT_TURNS: usize = 20;
pub(crate) const FLOW_TURNS: usize = 12;
pub(crate) const MAX_FLOW_DEPTH: usize = 2;
static CALL_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn call_id() -> String {
    CALL_ID.fetch_add(1, Ordering::Relaxed).to_string()
}

pub(crate) fn send(events: &Sender<Event>, event: Event) {
    let _ = events.send(event);
}

impl RigRuntime {
    /// Construct the Rig agent for a model invocation, including its typed tool registry.
    pub(crate) fn define_agent(
        &self,
        model: &str,
        depth: usize,
        allow_questions: bool,
    ) -> Result<Agent> {
        let client = rig::providers::deepseek::from_env()?;
        let mut preamble = self.agent.system_prompt.clone();
        for skill in &self.agent.skills {
            preamble.push_str(&format!(
                "\n\n## Skill: {}\n{}",
                skill.name, skill.instructions
            ));
        }
        let builder = AgentBuilder::new(client.chat(model)).preamble(preamble);
        Ok(define_tools(builder, self, depth, allow_questions).build())
    }

    pub(crate) fn model_run<'a>(
        &'a self,
        model: &'a str,
        prompt: &'a str,
        history: Option<Vec<Message>>,
        depth: usize,
        turns: usize,
        allow_questions: bool,
    ) -> Pin<Box<dyn Future<Output = Result<PromptResponse>> + Send + 'a>> {
        Box::pin(async move {
            let history_is_chat = history.is_some();
            let _activity = ActivityGuard::new(&self.events, format!("Thinking with {model}..."));
            let agent = self.define_agent(model, depth, allow_questions)?;
            let mut runner = agent.prompt(prompt).max_turns(turns);
            if let Some(history) = history {
                runner = runner.history(history);
            }
            let mut stream = runner.stream();
            while let Some(item) = stream.next().await {
                match item? {
                    MultiTurnStreamItem::StreamAssistantItem(Item::Event(StreamEvent::Text {
                        text,
                        ..
                    })) => send(&self.events, Event::TextDelta { text }),
                    MultiTurnStreamItem::StreamAssistantItem(Item::Event(
                        StreamEvent::Reasoning { text, .. },
                    )) if history_is_chat => send(&self.events, Event::ReasoningDelta { text }),
                    MultiTurnStreamItem::FinalResponse(response) => return Ok(response),
                    _ => {}
                }
            }
            bail!("Rig stream ended without a final response")
        })
    }

    pub(crate) fn started(&self, id: &str, name: &str, detail: &str) {
        send(
            &self.events,
            Event::ToolStarted {
                id: id.into(),
                name: name.into(),
                detail: detail.into(),
            },
        );
    }

    pub(crate) fn finished(&self, id: &str, result: &std::result::Result<String, ToolError>) {
        send(
            &self.events,
            Event::ToolFinished {
                id: id.into(),
                success: result.is_ok(),
                output: match result {
                    Ok(output) => output.clone(),
                    Err(error) => error.to_string(),
                },
            },
        );
    }
}
