use adk_core::{Content, Part, SessionId, UserId};
use adk_runner::Runner;
use adk_session::{CreateRequest, InMemorySessionService, SessionService};
use anyhow::{Result, bail};
use diffagent_core::{
    events::{ActivityGuard, Event, QuestionHandler},
    spec::LoadedAgent,
    tools::ToolHost,
};
use futures::StreamExt;
use std::{
    path::Path,
    sync::{Arc, mpsc::Sender},
};

use crate::{
    agent::{default_question_handler, define_agent, model},
    tools::emit,
};

// Types and structs -----------------------------------------------------------

/// Configure a chat before its ADK runner and tools are built.
pub struct ChatBuilder {
    agent: LoadedAgent,
    workspace: std::path::PathBuf,
    events: Sender<Event>,
    handler: QuestionHandler,
}

/// A live conversation: the runner and its in-memory session survive every submission.
pub struct Chat {
    runner: Runner,
    user: UserId,
    session: SessionId,
    events: Sender<Event>,
}

// Public API ------------------------------------------------------------------

impl ChatBuilder {
    pub fn with_question_handler(mut self, handler: QuestionHandler) -> Self {
        self.handler = handler;
        self
    }

    pub async fn build(self) -> Result<Chat> {
        Chat::new_with_question_handler(self.agent, &self.workspace, self.events, self.handler)
            .await
    }
}

impl Chat {
    pub fn builder(agent: LoadedAgent, workspace: &Path, events: Sender<Event>) -> ChatBuilder {
        ChatBuilder {
            agent,
            workspace: workspace.to_path_buf(),
            events,
            handler: default_question_handler(),
        }
    }

    pub async fn new(agent: LoadedAgent, workspace: &Path, events: Sender<Event>) -> Result<Self> {
        Self::new_with_question_handler(agent, workspace, events, default_question_handler()).await
    }

    pub async fn new_with_question_handler(
        agent: LoadedAgent,
        workspace: &Path,
        events: Sender<Event>,
        handler: QuestionHandler,
    ) -> Result<Self> {
        let host = ToolHost::new(workspace, agent.spec.tools.clone())?;
        let agent = Arc::new(agent);
        let llm = model(&agent.spec.model)?;
        let root = define_agent(&agent, host, llm, &events, true, handler, true)?;
        let sessions = Arc::new(InMemorySessionService::new()) as Arc<dyn SessionService>;
        let user = UserId::new("local-user")?;
        let session = SessionId::new("chat")?;
        sessions
            .create(CreateRequest {
                app_name: "diffagent-adk".into(),
                user_id: "local-user".into(),
                session_id: Some("chat".into()),
                state: Default::default(),
            })
            .await?;
        let runner = Runner::builder()
            .app_name("diffagent-adk")
            .agent(root)
            .session_service(sessions)
            .build()?;
        Ok(Self {
            runner,
            user,
            session,
            events,
        })
    }

    pub async fn submit(&self, prompt: &str) -> Result<String> {
        let stream = self
            .runner
            .run(
                self.user.clone(),
                self.session.clone(),
                Content::new("user").with_text(prompt),
            )
            .await?;
        drain(stream, &self.events, "coding", true).await
    }
}

// Helpers ---------------------------------------------------------------------

pub(crate) async fn drain(
    mut stream: adk_core::EventStream,
    tx: &Sender<Event>,
    author: &str,
    finish_turn: bool,
) -> Result<String> {
    let _activity = ActivityGuard::new(tx, "Thinking...");
    let mut response = String::new();
    while let Some(event) = stream.next().await {
        let event = event?;
        if event.author != author {
            continue;
        }
        let text = event
            .content()
            .map(|content| {
                content
                    .parts
                    .iter()
                    .filter_map(|part| match part {
                        Part::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>()
            })
            .unwrap_or_default();
        if event.llm_response.partial {
            if !text.is_empty() {
                emit(tx, Event::TextDelta { text: text.clone() });
            }
            if finish_turn && let Some(content) = event.content() {
                for part in &content.parts {
                    if let Part::Thinking { thinking, .. } = part
                        && !thinking.is_empty()
                    {
                        emit(
                            tx,
                            Event::ReasoningDelta {
                                text: thinking.clone(),
                            },
                        );
                    }
                }
            }
        }
        if event.is_final_response() && !text.is_empty() {
            response = text;
        }
    }
    if response.is_empty() {
        bail!("ADK agent returned no final text")
    }
    if finish_turn {
        emit(
            tx,
            Event::Finished {
                response: response.clone(),
            },
        );
    }
    Ok(response)
}
