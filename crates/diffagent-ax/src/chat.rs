use crate::{Engine, tools::bounded};
use anyhow::Result;
use diffagent_core::events::Event;

// Constants -------------------------------------------------------------------
const MAX_HISTORY: usize = 8;

// Public API ------------------------------------------------------------------
impl Engine {
    /// Chat is an Ax tool-using ReAct run. Its bounded transcript is passed to
    /// each new run; flows are never selected implicitly.
    pub fn chat(&self, history: &mut Vec<(String, String)>, request: &str) -> Result<String> {
        let mut context = self.instructions();
        context.push_str("\nUse workspace tools for file operations. To run a YAML flow, explicitly call workspace.run_flow with its name and arguments. The JavaScript actor cannot directly edit files.\n");
        for (user, answer) in history.iter().rev().take(MAX_HISTORY).rev() {
            context.push_str("\nPrevious user: ");
            context.push_str(&bounded(user));
            context.push_str("\nPrevious assistant: ");
            context.push_str(&bounded(answer));
        }
        context.push_str("\nCurrent user: ");
        context.push_str(&bounded(request));
        let response = self.generate(&self.agent.spec.model, &context, false, true)?;
        history.push((bounded(request), bounded(&response)));
        if history.len() > MAX_HISTORY {
            history.drain(..history.len() - MAX_HISTORY);
        }
        self.emit(Event::Finished {
            response: response.clone(),
        });
        Ok(response)
    }
}
