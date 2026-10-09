//! SDK-neutral observations and a broker for optional user questions.
//!
//! SDK libraries do not depend on a terminal. The UI chooses which events
//! to display and returns question answers through the broker.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

mod questions;
pub use questions::{QuestionAnswer, QuestionHandler, QuestionService};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Provisional assistant output. It is not necessarily the final answer.
    TextDelta {
        text: String,
    },
    /// Short activity label while a model or tool is still working. Empty clears it.
    Activity {
        label: String,
    },
    /// Provider-supplied reasoning text only; never synthesize private reasoning.
    ReasoningDelta {
        text: String,
    },
    /// A model tool is waiting for a user answer; choose an option, type text, or skip.
    QuestionAsked {
        id: String,
        question: String,
        options: Vec<String>,
    },
    ToolStarted {
        id: String,
        name: String,
        detail: String,
    },
    ToolFinished {
        id: String,
        success: bool,
        output: String,
    },
    /// Content may be replaced by a later preview with the same ID. No disk
    /// write should occur until the corresponding tool has been validated.
    WritePreview {
        id: String,
        path: String,
        /// Previous file content, if available, for an accurate provisional diff.
        original: Option<String>,
        content: String,
    },
    WriteCommitted {
        id: String,
        path: String,
    },
    FlowNodeStarted {
        flow: String,
        node: String,
        model: Option<String>,
    },
    FlowNodeFinished {
        flow: String,
        node: String,
        success: bool,
    },
    /// A final response, which supersedes any provisional text deltas.
    Finished {
        response: String,
    },
    Error {
        message: String,
    },
}

/// Clears a live loading label on success, error, or cancellation.
pub struct ActivityGuard(Sender<Event>);

impl ActivityGuard {
    pub fn new(events: &Sender<Event>, label: impl Into<String>) -> Self {
        let _ = events.send(Event::Activity {
            label: label.into(),
        });
        Self(events.clone())
    }
}

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        let _ = self.0.send(Event::Activity {
            label: String::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_label_clears_when_work_ends() -> Result<(), std::sync::mpsc::RecvError> {
        let (tx, rx) = std::sync::mpsc::channel();
        {
            let _activity = ActivityGuard::new(&tx, "Thinking...");
            assert_eq!(
                rx.recv()?,
                Event::Activity {
                    label: "Thinking...".into()
                }
            );
        }
        assert_eq!(
            rx.recv()?,
            Event::Activity {
                label: String::new()
            }
        );
        Ok(())
    }
}
