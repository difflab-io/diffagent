use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Sender},
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::events::Event;

const MAX_QUESTION_BYTES: usize = 4096;
const MAX_ANSWER_BYTES: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum QuestionAnswer {
    Answered { answer: String },
    Skipped { reason: String },
}

impl QuestionAnswer {
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self::Skipped {
            reason: reason.into(),
        }
    }

    /// A log-safe description. The full answer goes only to the model tool result.
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Answered { .. } => "question answered",
            Self::Skipped { .. } => "question skipped",
        }
    }
}

pub type QuestionHandler = Arc<dyn Fn(&str, &[String]) -> QuestionAnswer + Send + Sync>;

/// Correlates one model tool call with one user response without blocking the UI.
/// Dropping the UI calls `cancel_all` so pending tools do not hang forever.
#[derive(Clone)]
pub struct QuestionService {
    events: Sender<Event>,
    pending: Arc<Mutex<HashMap<String, Sender<Option<String>>>>>,
    next_id: Arc<AtomicU64>,
}

impl QuestionService {
    pub fn new(events: Sender<Event>) -> Self {
        Self {
            events,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn ask(&self, question: &str, options: &[String]) -> QuestionAnswer {
        if question.trim().is_empty()
            || question.len() > MAX_QUESTION_BYTES
            || options.len() > 4
            || options
                .iter()
                .any(|option| option.trim().is_empty() || option.len() > 256)
        {
            return QuestionAnswer::skipped("invalid question or options");
        }
        let id = format!("question-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id.clone(), tx);
        if self
            .events
            .send(Event::QuestionAsked {
                id: id.clone(),
                question: question.into(),
                options: options.to_vec(),
            })
            .is_err()
        {
            self.pending.lock().unwrap().remove(&id);
            return QuestionAnswer::skipped("question UI unavailable");
        }
        let result = rx.recv_timeout(Duration::from_secs(600));
        self.pending.lock().unwrap().remove(&id);
        match result {
            Ok(Some(answer)) if !answer.trim().is_empty() && answer.len() <= MAX_ANSWER_BYTES => {
                QuestionAnswer::Answered { answer }
            }
            Ok(Some(_)) => QuestionAnswer::skipped("empty or oversized answer"),
            Ok(None) => QuestionAnswer::skipped("user skipped the question"),
            Err(mpsc::RecvTimeoutError::Timeout) => QuestionAnswer::skipped("question timed out"),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                QuestionAnswer::skipped("question UI closed")
            }
        }
    }

    pub fn answer(&self, id: &str, answer: Option<String>) -> bool {
        self.pending
            .lock()
            .unwrap()
            .remove(id)
            .is_some_and(|tx| tx.send(answer).is_ok())
    }

    pub fn cancel_all(&self) {
        self.pending.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_correlated_and_skip_is_explicit() {
        let (tx, rx) = mpsc::channel();
        let service = QuestionService::new(tx);
        let worker = service.clone();
        let first = std::thread::spawn(move || worker.ask("Which?", &["A".into(), "B".into()]));
        let id = match rx.recv().unwrap() {
            Event::QuestionAsked {
                id,
                question,
                options,
            } => {
                assert_eq!(question, "Which?");
                assert_eq!(options, ["A", "B"]);
                id
            }
            other => panic!("unexpected event: {other:?}"),
        };
        assert!(!service.answer("wrong-id", Some("A".into())));
        assert!(service.answer(&id, Some("B".into())));
        assert_eq!(
            first.join().unwrap(),
            QuestionAnswer::Answered { answer: "B".into() }
        );
        let worker = service.clone();
        let second = std::thread::spawn(move || worker.ask("Skip?", &[]));
        let id = match rx.recv().unwrap() {
            Event::QuestionAsked { id, .. } => id,
            other => panic!("unexpected event: {other:?}"),
        };
        assert!(service.answer(&id, None));
        assert!(matches!(
            second.join().unwrap(),
            QuestionAnswer::Skipped { .. }
        ));
    }

    #[test]
    fn closing_the_ui_unblocks_pending_question() {
        let (tx, rx) = mpsc::channel();
        let service = QuestionService::new(tx);
        let worker = service.clone();
        let thread = std::thread::spawn(move || worker.ask("Question?", &[]));
        rx.recv().unwrap();
        service.cancel_all();
        assert_eq!(
            thread.join().unwrap(),
            QuestionAnswer::skipped("question UI closed")
        );
    }
}
