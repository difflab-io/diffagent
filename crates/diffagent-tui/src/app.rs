pub use crate::tool_activity::ToolKind;
use crate::{editor::Editor, question_dialog::QuestionDialog, transcript::Transcript};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
#[cfg(test)]
use diffagent_core::events::QuestionAnswer;
use diffagent_core::events::{Event as AgentEvent, QuestionService};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{collections::VecDeque, sync::mpsc::Sender};

/// Observations become entries in one scrollable conversation, not side panes.
/// A provisional write is display-only; SDK tools perform the atomic commit.
#[derive(Debug)]
pub enum UiEvent {
    AssistantDelta(String),
    ReasoningDelta(String),
    Activity(String),
    QuestionAsked {
        id: String,
        question: String,
        options: Vec<String>,
    },
    AssistantDone(String),
    ToolStarted {
        id: String,
        kind: ToolKind,
    },
    ToolCompleted {
        id: String,
        result: Result<String, String>,
    },
    ProvisionalWrite {
        id: String,
        path: String,
        original: Option<String>,
        content: String,
    },
    CommittedWrite {
        id: String,
        path: String,
    },
    Error(String),
    Status(String),
}

impl From<AgentEvent> for UiEvent {
    fn from(event: AgentEvent) -> Self {
        match event {
            AgentEvent::TextDelta { text } => Self::AssistantDelta(text),
            AgentEvent::ReasoningDelta { text } => Self::ReasoningDelta(text),
            AgentEvent::Activity { label } => Self::Activity(label),
            AgentEvent::QuestionAsked {
                id,
                question,
                options,
            } => Self::QuestionAsked {
                id,
                question,
                options,
            },
            AgentEvent::Finished { response } => Self::AssistantDone(response),
            AgentEvent::Error { message } => Self::Error(message),
            AgentEvent::ToolStarted { id, name, detail } => {
                let kind = match name.as_str() {
                    "read_file" => ToolKind::Read { path: detail },
                    "write_file" => ToolKind::Write { path: detail },
                    "run_task" => ToolKind::NamedTask { name: detail },
                    _ => ToolKind::Other { name, detail },
                };
                Self::ToolStarted { id, kind }
            }
            AgentEvent::ToolFinished {
                id,
                success,
                output,
            } => Self::ToolCompleted {
                id,
                result: if success { Ok(output) } else { Err(output) },
            },
            AgentEvent::WritePreview {
                id,
                path,
                original,
                content,
            } => Self::ProvisionalWrite {
                id,
                path,
                original,
                content,
            },
            AgentEvent::WriteCommitted { id, path } => Self::CommittedWrite { id, path },
            AgentEvent::FlowNodeStarted { flow, node, model } => Self::Status(format!(
                "{flow}: {node} ({})",
                model.as_deref().unwrap_or("task")
            )),
            AgentEvent::FlowNodeFinished {
                flow,
                node,
                success,
            } => Self::Status(format!(
                "{flow}: {node} {}",
                if success { "done" } else { "failed" }
            )),
        }
    }
}

#[derive(Debug)]
struct Viewer {
    entry: usize,
    scroll: u16,
    copy_feedback: Option<String>,
}

#[derive(Default)]
pub struct App {
    editor: Editor,
    transcript: Transcript,
    viewer: Option<Viewer>,
    questions: VecDeque<QuestionDialog>,
    question_service: Option<QuestionService>,
    quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_questions(service: QuestionService) -> Self {
        Self {
            question_service: Some(service),
            ..Self::default()
        }
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Copy the full modal entry, or the full chronological chat without display caps.
    pub fn copy_text(&self) -> Option<(String, &'static str)> {
        let (text, label) = if let Some(viewer) = &self.viewer {
            (self.transcript.copy_entry(viewer.entry)?, "full output")
        } else {
            (self.transcript.copy_text(), "chat transcript")
        };
        (!text.is_empty()).then_some((text, label))
    }

    pub fn copy_feedback(&mut self, message: String) {
        if let Some(viewer) = &mut self.viewer {
            viewer.copy_feedback = Some(message.clone());
        }
        self.transcript.status(message);
    }

    pub fn paste(&mut self, text: &str) {
        if let Some(question) = self.questions.front_mut() {
            question.paste(text);
            return;
        }
        if !self.editor.paste(text) {
            self.transcript
                .error("Paste rejected: message exceeds 64 KB".into());
        }
    }

    pub fn apply(&mut self, event: UiEvent) {
        match event {
            UiEvent::AssistantDelta(text) => {
                self.transcript.activity("Writing response...".into());
                self.transcript.delta(text);
            }
            UiEvent::ReasoningDelta(text) => self.transcript.reasoning_delta(text),
            UiEvent::Activity(label) => self.transcript.activity(label),
            UiEvent::QuestionAsked {
                id,
                question,
                options,
            } => {
                self.transcript.status(format!("Agent asks: {question}"));
                self.questions
                    .push_back(QuestionDialog::new(id, question, options));
            }
            UiEvent::AssistantDone(response) => {
                self.transcript.activity(String::new());
                self.transcript.finalize(response);
            }
            UiEvent::ToolStarted { id, kind } => {
                self.transcript.activity("Running tool...".into());
                self.transcript.tool_started(id, kind);
            }
            UiEvent::ToolCompleted { id, result } => {
                self.transcript.tool_completed(&id, result);
                self.transcript.activity("Thinking...".into());
            }
            UiEvent::ProvisionalWrite {
                id,
                path,
                original,
                content,
            } => {
                self.transcript.provisional(id, path, original, content);
            }
            UiEvent::CommittedWrite { id, path } => self.transcript.committed(&id, &path),
            UiEvent::Error(error) => self.transcript.error(error),
            UiEvent::Status(status) => self.transcript.status(status),
        }
    }

    /// Returns false if the submission channel has closed.
    pub fn handle_key(&mut self, key: KeyEvent, submissions: &Sender<String>) -> bool {
        if key.kind != KeyEventKind::Press {
            return true;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return true;
        }
        if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.transcript.toggle_reasoning();
            return true;
        }
        if let Some(question) = self.questions.front_mut() {
            if let Some(answer) = question.key(key) {
                self.answer_question(answer);
            }
            return true;
        }
        if let Some(viewer) = &mut self.viewer {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.viewer = None,
                KeyCode::Up => viewer.scroll = viewer.scroll.saturating_sub(1),
                KeyCode::Down => viewer.scroll = viewer.scroll.saturating_add(1),
                KeyCode::PageUp => viewer.scroll = viewer.scroll.saturating_sub(10),
                KeyCode::PageDown => viewer.scroll = viewer.scroll.saturating_add(10),
                _ => {}
            }
            return true;
        }
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => self.quit = true,
            (KeyCode::Char('o'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.viewer = self.transcript.newest_expandable().map(|entry| Viewer {
                    entry,
                    scroll: 0,
                    copy_feedback: None,
                });
            }
            (KeyCode::Up | KeyCode::PageUp, _) => {
                self.transcript
                    .scroll_up(if key.code == KeyCode::PageUp { 10 } else { 1 });
            }
            (KeyCode::Down | KeyCode::PageDown, _) => {
                self.transcript
                    .scroll_down(if key.code == KeyCode::PageDown { 10 } else { 1 });
            }
            _ => {
                if let Some(message) = self.editor.handle(key) {
                    if submissions.send(message.clone()).is_err() {
                        return false;
                    }
                    self.transcript.user(message);
                }
            }
        }
        true
    }

    fn answer_question(&mut self, answer: Option<String>) {
        if let Some(question) = self.questions.pop_front() {
            if let Some(service) = &self.question_service {
                service.answer(&question.id, answer.clone());
            }
            self.transcript.status(if answer.is_some() {
                "Question answered".into()
            } else {
                "Question skipped".into()
            });
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let Some(question) = self.questions.front()
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            if let Some(answer) = question.click(mouse.column, mouse.row) {
                self.answer_question(answer);
            }
            return;
        }
        if !self.questions.is_empty() {
            return;
        }
        if let Some(viewer) = &mut self.viewer {
            match mouse.kind {
                MouseEventKind::ScrollUp => viewer.scroll = viewer.scroll.saturating_sub(3),
                MouseEventKind::ScrollDown => viewer.scroll = viewer.scroll.saturating_add(3),
                MouseEventKind::Down(MouseButton::Left) => self.viewer = None,
                _ => {}
            }
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.viewer = self
                    .transcript
                    .target_at(mouse.column, mouse.row)
                    .map(|entry| Viewer {
                        entry,
                        scroll: 0,
                        copy_feedback: None,
                    });
            }
            MouseEventKind::ScrollUp => self.transcript.scroll_up(3),
            MouseEventKind::ScrollDown => self.transcript.scroll_down(3),
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(4), Constraint::Length(5)])
            .split(frame.area());
        self.transcript.render(frame, areas[0]);
        self.editor.render(frame, areas[1]);
        if let Some(viewer) = &self.viewer
            && let Some((title, lines)) = self.transcript.modal_lines(viewer.entry)
        {
            let screen = frame.area();
            let width = (screen.width.saturating_mul(9) / 10).max(1);
            let height = (screen.height.saturating_mul(9) / 10).max(1);
            let area = Rect::new(
                screen.x + (screen.width - width) / 2,
                screen.y + (screen.height - height) / 2,
                width,
                height,
            );
            let inner_width = width.saturating_sub(2).max(1) as usize;
            let total = lines
                .iter()
                .map(|line| line.width().max(1).div_ceil(inner_width))
                .sum::<usize>();
            let max_scroll = total
                .saturating_sub(height.saturating_sub(2) as usize)
                .min(u16::MAX as usize) as u16;
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new(lines)
                    .block(
                        Block::default()
                            .title(format!(
                                "{title} · Ctrl+Y copy · Esc closes{}",
                                viewer
                                    .copy_feedback
                                    .as_deref()
                                    .map_or(String::new(), |status| format!(" · {status}"))
                            ))
                            .borders(Borders::ALL),
                    )
                    .wrap(Wrap { trim: false })
                    .scroll((viewer.scroll.min(max_scroll), 0)),
                area,
            );
        }
        if let Some(question) = self.questions.front() {
            question.render(frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn screen(terminal: &mut Terminal<TestBackend>, app: &App) -> String {
        terminal.draw(|frame| app.render(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn submits_unicode_input_through_channel() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = App::new();
        for character in "hé👋".chars() {
            assert!(app.handle_key(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                &tx
            ));
        }
        assert!(app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &tx));
        assert_eq!(rx.try_recv().unwrap(), "hé👋");
    }

    #[test]
    fn tools_and_diffs_render_inline_not_in_separate_panes() {
        let mut app = App::new();
        app.apply(UiEvent::AssistantDelta("I will make a change.".into()));
        app.apply(UiEvent::ToolStarted {
            id: "1".into(),
            kind: ToolKind::Write {
                path: "src/lib.rs".into(),
            },
        });
        app.apply(UiEvent::ProvisionalWrite {
            id: "preview-1".into(),
            path: "src/lib.rs".into(),
            original: Some("pub fn old() {}".into()),
            content: "pub fn test() {}".into(),
        });
        let mut terminal = Terminal::new(TestBackend::new(110, 32)).unwrap();
        let pending = screen(&mut terminal, &app);
        assert!(pending.contains("I will make a change."));
        assert!(pending.contains("write src/lib.rs"));
        assert!(pending.contains("provisional"));
        assert!(pending.contains("pub fn test"));
        assert!(!pending.contains("Write preview"));
        assert!(!pending.contains("Tools"));
        app.apply(UiEvent::CommittedWrite {
            id: "preview-1".into(),
            path: "src/lib.rs".into(),
        });
        app.apply(UiEvent::ToolCompleted {
            id: "1".into(),
            result: Ok("wrote file".into()),
        });
        app.apply(UiEvent::AssistantDone("Done.".into()));
        let committed = screen(&mut terminal, &app);
        assert!(committed.contains("committed"));
        assert!(committed.contains("[done] write src/lib.rs"));
        assert!(committed.contains("Done."));
    }

    #[test]
    fn clicking_capped_stream_opens_live_full_modal() {
        let mut app = App::new();
        let code = (0..18)
            .map(|n| format!("let n{n} = {n};"))
            .collect::<Vec<_>>()
            .join("\n");
        app.apply(UiEvent::AssistantDelta(format!("```rust\n{code}\n```")));
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        let inline = screen(&mut terminal, &app);
        assert!(inline.contains("6 more lines"));
        assert!(!inline.contains("n17"));
        let row = terminal
            .backend()
            .buffer()
            .content()
            .chunks(100)
            .position(|cells| {
                cells
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>()
                    .contains("6 more lines")
            })
            .expect("expand hint is visible") as u16;
        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 15,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert!(screen(&mut terminal, &app).contains("n17"));
        app.apply(UiEvent::AssistantDelta("\nupdated live".into()));
        assert!(screen(&mut terminal, &app).contains("updated live"));
        let (tx, _rx) = std::sync::mpsc::channel();
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &tx);
        assert!(!screen(&mut terminal, &app).contains("n17"));
    }

    #[test]
    fn copy_targets_full_chat_or_full_open_output() {
        let mut app = App::new();
        let (tx, _rx) = std::sync::mpsc::channel();
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), &tx);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &tx);
        let code = (0..20)
            .map(|n| format!("go_line_{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.apply(UiEvent::AssistantDelta(format!("```go\n{code}\n```")));
        let (chat, label) = app.copy_text().unwrap();
        assert_eq!(label, "chat transcript");
        assert!(chat.contains("You:\nh"));
        assert!(chat.contains("go_line_19"));
        app.handle_key(
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            &tx,
        );
        let (output, label) = app.copy_text().unwrap();
        assert_eq!(label, "full output");
        assert!(output.contains("go_line_19"));
        assert!(!output.contains("You:\nh"));
        app.copy_feedback("Copied full output".into());
        let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
        assert!(screen(&mut terminal, &app).contains("Copied full output"));
    }

    #[test]
    fn plain_chat_answer_can_open_and_copy_by_itself() {
        let mut app = App::new();
        app.apply(UiEvent::AssistantDone("This is a plain answer.".into()));
        let (tx, _rx) = std::sync::mpsc::channel();
        app.handle_key(
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            &tx,
        );
        assert_eq!(
            app.copy_text(),
            Some(("This is a plain answer.".into(), "full output"))
        );
    }

    #[test]
    fn narrow_terminal_modal_scrolls_to_last_streamed_line() {
        let mut app = App::new();
        let code = (0..30)
            .map(|n| format!("let n{n} = {n};"))
            .collect::<Vec<_>>()
            .join("\n");
        app.apply(UiEvent::AssistantDelta(format!("```rust\n{code}\n```")));
        let mut terminal = Terminal::new(TestBackend::new(52, 15)).unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        app.handle_key(
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            &tx,
        );
        assert!(screen(&mut terminal, &app).contains("full streamed output"));
        for _ in 0..5 {
            app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE), &tx);
        }
        assert!(screen(&mut terminal, &app).contains("n29"));
    }

    #[test]
    fn clicking_write_preview_shows_all_changed_lines() {
        let mut app = App::new();
        let content = (0..20)
            .map(|n| format!("line_{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.apply(UiEvent::ProvisionalWrite {
            id: "p".into(),
            path: "src/lib.rs".into(),
            original: None,
            content,
        });
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        let inline = screen(&mut terminal, &app);
        assert!(inline.contains("8 more lines"));
        assert!(!inline.contains("line_19"));
        let row = terminal
            .backend()
            .buffer()
            .content()
            .chunks(100)
            .position(|cells| {
                cells
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>()
                    .contains("8 more lines")
            })
            .expect("preview expand hint is visible") as u16;
        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 15,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert!(screen(&mut terminal, &app).contains("line_19"));
    }

    #[test]
    fn question_modal_answers_or_skips_without_sending_chat_input() {
        let (events_tx, events_rx) = std::sync::mpsc::channel();
        let service = QuestionService::new(events_tx);
        let mut app = App::with_questions(service.clone());
        let worker = service.clone();
        let answer = std::thread::spawn(move || {
            worker.ask("Which approach?", &["Fast".into(), "Safe".into()])
        });
        app.apply(UiEvent::from(events_rx.recv().unwrap()));
        let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
        assert!(screen(&mut terminal, &app).contains("Which approach?"));
        let (chat_tx, chat_rx) = std::sync::mpsc::channel();
        app.handle_key(
            KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
            &chat_tx,
        );
        assert_eq!(
            answer.join().unwrap(),
            QuestionAnswer::Answered {
                answer: "Safe".into()
            }
        );
        assert!(chat_rx.try_recv().is_err());
        let worker = service.clone();
        let skipped = std::thread::spawn(move || worker.ask("Skip?", &[]));
        app.apply(UiEvent::from(events_rx.recv().unwrap()));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &chat_tx);
        assert!(matches!(
            skipped.join().unwrap(),
            QuestionAnswer::Skipped { .. }
        ));
        assert!(!app.should_quit());
    }

    #[test]
    fn loading_and_provider_reasoning_can_be_shown_or_hidden() {
        let mut app = App::new();
        app.apply(UiEvent::Activity("Thinking with model...".into()));
        app.apply(UiEvent::ReasoningDelta("provider reasoning text".into()));
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let hidden = screen(&mut terminal, &app);
        assert!(hidden.contains("Thinking with model..."));
        assert!(hidden.contains("Provider reasoning hidden"));
        assert!(!hidden.contains("provider reasoning text"));
        let (tx, _rx) = std::sync::mpsc::channel();
        app.handle_key(
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
            &tx,
        );
        assert!(screen(&mut terminal, &app).contains("provider reasoning text"));
        app.handle_key(
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
            &tx,
        );
        assert!(!screen(&mut terminal, &app).contains("provider reasoning text"));
        app.apply(UiEvent::AssistantDone("Done".into()));
        assert!(!screen(&mut terminal, &app).contains("Thinking with model..."));
    }

    #[test]
    fn terminal_colors_headings_and_fenced_code_without_resizing() {
        use ratatui::style::Color;

        let mut app = App::new();
        app.apply(UiEvent::AssistantDone(
            "# Colored\n\n```rust\nfn demo() {}\n```".into(),
        ));
        let mut terminal = Terminal::new(TestBackend::new(80, 18)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        let cells = terminal.backend().buffer().content();
        let heading = cells.windows(7).find(|window| {
            window.iter().map(|cell| cell.symbol()).collect::<String>() == "Colored"
        });
        assert_eq!(heading.map(|window| window[0].fg), Some(Color::LightCyan));
        let keyword = cells
            .windows(2)
            .find(|window| window[0].symbol() == "f" && window[1].symbol() == "n");
        assert_eq!(keyword.map(|window| window[0].fg), Some(Color::Magenta));
    }

    #[test]
    fn up_and_down_scroll_the_whole_conversation() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut app = App::new();
        app.apply(UiEvent::AssistantDone(
            (0..40)
                .map(|n| format!("line {n:02}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).unwrap();
        let bottom = screen(&mut terminal, &app);
        assert!(app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), &tx));
        let above = screen(&mut terminal, &app);
        assert_ne!(above, bottom);
        app.apply(UiEvent::Status("new tool activity".into()));
        assert_eq!(screen(&mut terminal, &app), above);
        assert!(app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &tx));
        assert!(app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE), &tx));
        assert!(screen(&mut terminal, &app).contains("new tool activity"));
    }
}
