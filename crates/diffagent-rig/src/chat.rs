use anyhow::{Context, Result};
use diffagent_core::events::Event;
use rig::completion::Message;

use crate::agent::{CHAT_TURNS, RigRuntime, send};

// Public API ------------------------------------------------------------------
impl RigRuntime {
    /// One chat submission. History contains the actual accepted Rig messages,
    /// including tool calls and results, not just flattened assistant text.
    /// On failure the caller's history is unchanged.
    pub async fn chat(&self, prompt: &str, history: &mut Vec<Message>) -> Result<String> {
        let response = self
            .model_run(
                &self.agent.spec.model,
                prompt,
                Some(history.clone()),
                0,
                CHAT_TURNS,
                true,
            )
            .await?;
        let messages = response
            .messages
            .context("Rig did not return a conversation transcript")?;
        history.extend(messages);
        send(
            &self.events,
            Event::Finished {
                response: response.output.clone(),
            },
        );
        Ok(response.output)
    }
}
