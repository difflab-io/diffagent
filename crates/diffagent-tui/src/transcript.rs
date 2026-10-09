use std::cell::Cell;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    markdown,
    sanitize::{copyable, visible},
    tool_activity::{ToolItem, ToolKind},
    write_preview::WritePreview,
};

#[derive(Default, Debug)]
pub struct Transcript {
    entries: Vec<Entry>,
    scroll: Scroll,
    /// Scroll limit from the last draw, used to anchor the viewport on Up.
    last_max_scroll: Cell<u16>,
    last_area: Cell<Rect>,
    activity: Option<String>,
    reasoning_visible: bool,
    frame_tick: Cell<u8>,
}

#[derive(Default, Debug)]
enum Scroll {
    #[default]
    Follow,
    Fixed(u16),
}

#[derive(Debug)]
enum Entry {
    User(String),
    Assistant { text: String, complete: bool },
    Tool(ToolItem),
    ToolResult(ToolItem),
    Reasoning(String),
    Preview(WritePreview),
    WriteCommit(String),
    Error(String),
    Status(String),
}

impl Transcript {
    pub fn user(&mut self, text: String) {
        self.entries.push(Entry::User(text));
        self.scroll = Scroll::Follow;
    }

    pub fn activity(&mut self, label: String) {
        self.activity = if label.is_empty() { None } else { Some(label) };
    }

    pub fn reasoning_delta(&mut self, text: String) {
        if let Some(Entry::Reasoning(existing)) = self.entries.last_mut() {
            existing.push_str(&text);
        } else {
            self.entries.push(Entry::Reasoning(text));
        }
    }

    pub fn toggle_reasoning(&mut self) {
        self.reasoning_visible = !self.reasoning_visible;
    }

    pub fn delta(&mut self, text: String) {
        if let Some(Entry::Assistant {
            text: last,
            complete: false,
        }) = self.entries.last_mut()
        {
            last.push_str(&text);
        } else {
            self.entries.push(Entry::Assistant {
                text,
                complete: false,
            });
        }
    }

    pub fn finalize(&mut self, response: String) {
        if let Some(Entry::Assistant { text, complete }) = self.entries.last_mut()
            && !*complete
        {
            *text = response;
            *complete = true;
            return;
        }
        self.entries.push(Entry::Assistant {
            text: response,
            complete: true,
        });
    }

    pub fn tool_started(&mut self, id: String, kind: ToolKind) {
        self.entries.push(Entry::Tool(ToolItem::new(id, kind)));
    }

    pub fn tool_completed(&mut self, id: &str, result: Result<String, String>) {
        let kind = self.entries.iter_mut().rev().find_map(|entry| match entry {
            Entry::Tool(item) if item.id == id && item.result.is_none() => {
                // Only the completion event carries output; the start remains in place.
                item.result = Some(Ok(String::new()));
                Some(item.kind.clone())
            }
            _ => None,
        });
        if let Some(kind) = kind {
            let mut completed = ToolItem::new(id.to_owned(), kind);
            completed.result = Some(result);
            self.entries.push(Entry::ToolResult(completed));
        } else {
            self.status(format!("tool {id} finished without a start event"));
        }
    }

    pub fn provisional(
        &mut self,
        id: String,
        path: String,
        original: Option<String>,
        content: String,
    ) {
        self.entries.push(Entry::Preview(WritePreview::new(
            id, path, original, content,
        )));
    }

    pub fn committed(&mut self, id: &str, path: &str) {
        if let Some(Entry::Preview(preview)) = self.entries.iter_mut().rev().find(
            |entry| matches!(entry, Entry::Preview(preview) if preview.id == id && preview.path == path && !preview.committed),
        ) {
            preview.committed = true;
            self.entries.push(Entry::WriteCommit(path.to_owned()));
        } else {
            self.status(format!("write committed: {path}"));
        }
    }

    pub fn error(&mut self, text: String) {
        self.entries.push(Entry::Error(text));
    }

    pub fn status(&mut self, text: String) {
        self.entries.push(Entry::Status(text));
    }

    pub fn scroll_up(&mut self, lines: u16) {
        let max = self.last_max_scroll.get();
        if max == 0 {
            return;
        }
        self.scroll = Scroll::Fixed(match self.scroll {
            Scroll::Follow => max.saturating_sub(lines),
            Scroll::Fixed(top) => top.saturating_sub(lines),
        });
    }

    pub fn scroll_down(&mut self, lines: u16) {
        if let Scroll::Fixed(top) = self.scroll {
            let next = top.saturating_add(lines);
            self.scroll = if next >= self.last_max_scroll.get() {
                Scroll::Follow
            } else {
                Scroll::Fixed(next)
            };
        }
    }

    fn lines(&self) -> Vec<Line<'static>> {
        self.lines_with_targets().0
    }

    fn lines_with_targets(&self) -> (Vec<Line<'static>>, Vec<Option<usize>>) {
        let mut lines = Vec::new();
        let mut targets = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            let start = lines.len();
            match entry {
                Entry::User(text) => message(&mut lines, "You", Color::Cyan, text),
                Entry::Assistant { text, .. } if !text.is_empty() => {
                    message(&mut lines, "Assistant", Color::Green, text);
                }
                Entry::Assistant { .. } => {}
                Entry::Tool(item) => lines.push(item.started_line()),
                Entry::ToolResult(item) => lines.extend(item.lines()),
                Entry::Reasoning(text) if self.reasoning_visible => {
                    message(&mut lines, "Provider reasoning", Color::Yellow, text);
                }
                Entry::Reasoning(_) => lines.push(Line::styled(
                    "  · Provider reasoning hidden · Ctrl+T to show",
                    Style::default().fg(Color::DarkGray),
                )),
                Entry::Preview(preview) => {
                    lines.extend(preview.lines());
                    lines.push(Line::default());
                }
                Entry::WriteCommit(path) => lines.push(Line::from(vec![
                    Span::styled(
                        "  [committed] ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(visible(path), Style::default().fg(Color::Cyan)),
                ])),
                Entry::Error(error) => {
                    lines.push(Line::styled(
                        format!("Error: {}", visible(error)),
                        Style::default().fg(Color::Red),
                    ));
                    lines.push(Line::default());
                }
                Entry::Status(text) => lines.push(Line::styled(
                    format!("  · {}", visible(text)),
                    Style::default().fg(Color::DarkGray),
                )),
            }
            let target = match entry {
                Entry::Assistant { text, .. } if !text.is_empty() => Some(index),
                Entry::Preview(_) | Entry::ToolResult(_) => Some(index),
                _ => None,
            };
            targets.extend(std::iter::repeat_n(target, lines.len() - start));
        }
        if let Some(label) = &self.activity {
            const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let tick = self.frame_tick.get() as usize;
            lines.push(Line::styled(
                format!("  {} {}", SPINNER[tick % SPINNER.len()], visible(label)),
                Style::default().fg(Color::Yellow),
            ));
        }
        (lines, targets)
    }

    /// Copy the entire raw entry, not the shortened lines used for display.
    pub fn copy_entry(&self, index: usize) -> Option<String> {
        match self.entries.get(index)? {
            Entry::Assistant { text, .. } => Some(copyable(text)),
            Entry::Preview(preview) => Some(joined_lines(preview.full_lines())),
            Entry::ToolResult(item) => item
                .result
                .as_ref()
                .map(|result| copyable(result.as_ref().unwrap_or_else(|error| error))),
            _ => None,
        }
    }

    /// Copy every visible transcript entry, including full tool output and diffs.
    pub fn copy_text(&self) -> String {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry {
                Entry::User(text) => Some(format!("You:\n{}", copyable(text))),
                Entry::Assistant { text, .. } if !text.is_empty() => {
                    Some(format!("Assistant:\n{}", copyable(text)))
                }
                Entry::Assistant { .. } => None,
                Entry::Tool(item) => Some(line_text(&item.started_line())),
                Entry::ToolResult(item) => Some(format!(
                    "{}\n{}",
                    line_text(&item.line()),
                    self.copy_entry(index).unwrap_or_default()
                )),
                Entry::Reasoning(text) if self.reasoning_visible => {
                    Some(format!("Provider reasoning:\n{}", copyable(text)))
                }
                Entry::Reasoning(_) => None,
                Entry::Preview(_) => self.copy_entry(index),
                Entry::WriteCommit(path) => Some(format!("[committed] {}", copyable(path))),
                Entry::Error(text) => Some(format!("Error: {}", copyable(text))),
                Entry::Status(text) => Some(format!("· {}", copyable(text))),
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn modal_lines(&self, index: usize) -> Option<(String, Vec<Line<'static>>)> {
        match self.entries.get(index)? {
            Entry::Assistant { text, .. } => Some((
                "Assistant · full streamed output".into(),
                markdown::render(text),
            )),
            Entry::Preview(preview) => Some((
                format!("{} · full write diff", visible(&preview.path)),
                preview.full_lines(),
            )),
            Entry::ToolResult(item) => {
                Some(("Tool result · full output".into(), item.full_lines()))
            }
            _ => None,
        }
    }

    pub fn newest_expandable(&self) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, entry)| match entry {
                Entry::Assistant { text, .. } if !text.is_empty() => Some(index),
                Entry::Preview(_) | Entry::ToolResult(_) => Some(index),
                _ => None,
            })
    }

    pub fn target_at(&self, x: u16, y: u16) -> Option<usize> {
        let area = self.last_area.get();
        if x <= area.x
            || x >= area.right().saturating_sub(1)
            || y <= area.y
            || y >= area.bottom().saturating_sub(1)
        {
            return None;
        }
        let (lines, targets) = self.lines_with_targets();
        let width = area.width.saturating_sub(2).max(1) as usize;
        let scroll = match self.scroll {
            Scroll::Follow => self.last_max_scroll.get(),
            Scroll::Fixed(top) => top.min(self.last_max_scroll.get()),
        } as usize;
        let row = scroll + (y - area.y - 1) as usize;
        let mut top = 0;
        for (line, target) in lines.iter().zip(targets) {
            let height = line.width().max(1).div_ceil(width);
            if row < top + height {
                return target;
            }
            top += height;
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        self.frame_tick.set(self.frame_tick.get().wrapping_add(1));
        self.last_area.set(area);
        let lines = self.lines();
        let inner = area.height.saturating_sub(2);
        let width = area.width.saturating_sub(2).max(1) as usize;
        let total = lines
            .iter()
            .map(|line| line.width().max(1).div_ceil(width))
            .sum::<usize>()
            .min(u16::MAX as usize) as u16;
        let max_scroll = total.saturating_sub(inner);
        self.last_max_scroll.set(max_scroll);
        let scroll = match self.scroll {
            Scroll::Follow => max_scroll,
            Scroll::Fixed(top) => top.min(max_scroll),
        };
        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title("Chat · Ctrl+Y copy · ↑/↓ scroll · Ctrl+O full · Ctrl+T reasoning")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            area,
        );
    }
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn joined_lines(lines: Vec<Line<'_>>) -> String {
    lines.iter().map(line_text).collect::<Vec<_>>().join("\n")
}

fn message(lines: &mut Vec<Line<'static>>, label: &str, color: Color, text: &str) {
    lines.push(Line::from(vec![Span::styled(
        label.to_owned(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));
    lines.extend(markdown::render_inline(text));
    lines.push(Line::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_and_write_events_stay_in_conversation_order() {
        let mut transcript = Transcript::default();
        transcript.user("make a game".into());
        transcript.tool_started(
            "tool-1".into(),
            ToolKind::Write {
                path: "game.lua".into(),
            },
        );
        transcript.provisional(
            "preview-1".into(),
            "game.lua".into(),
            None,
            "local x = 1".into(),
        );
        transcript.committed("preview-1", "game.lua");
        transcript.tool_completed("tool-1", Ok("wrote game.lua".into()));
        transcript.finalize("Done!".into());
        let lines = transcript
            .lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let index = |needle: &str| {
            lines
                .iter()
                .position(|line| line.contains(needle))
                .expect("event appears in transcript")
        };
        assert!(index("You") < index("write game.lua"));
        assert!(index("write game.lua") < index("committed"));
        assert!(index("committed") < index("Done!"));
    }

    #[test]
    fn completions_follow_intervening_events_in_arrival_order() {
        let mut transcript = Transcript::default();
        transcript.tool_started(
            "a".into(),
            ToolKind::NamedTask {
                name: "test".into(),
            },
        );
        transcript.status("between start and result".into());
        transcript.tool_completed("a", Ok("tests passed".into()));
        transcript.provisional(
            "p".into(),
            "src/lib.rs".into(),
            None,
            "pub fn go() {}".into(),
        );
        transcript.status("between proposal and commit".into());
        transcript.committed("p", "src/lib.rs");
        let lines = transcript
            .lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let index = |needle: &str| {
            lines
                .iter()
                .position(|line| line.contains(needle))
                .expect("event appears in transcript")
        };
        assert!(index("[started]") < index("between start and result"));
        assert!(index("between start and result") < index("[done]"));
        assert!(index("[done]") < index("provisional"));
        assert!(index("provisional") < index("between proposal and commit"));
        assert!(index("between proposal and commit") < index("[committed]"));
    }

    #[test]
    fn tool_result_lines_are_inline_and_bounded() {
        let mut transcript = Transcript::default();
        transcript.tool_started(
            "task-1".into(),
            ToolKind::NamedTask {
                name: "test".into(),
            },
        );
        let output = (0..15)
            .map(|index| format!("result {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        transcript.tool_completed("task-1", Ok(output));
        let rendered = transcript
            .lines()
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(rendered.contains("result 0"));
        assert!(rendered.contains("result 11"));
        assert!(rendered.contains("3 more lines"));
        assert!(!rendered.contains("result 14"));
    }

    #[test]
    fn copy_includes_complete_outputs_but_not_hidden_reasoning() {
        let mut transcript = Transcript::default();
        transcript.user("show the Go code".into());
        let code = (0..20)
            .map(|n| format!("line_{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        transcript.delta(format!("```go\n{code}\n```"));
        transcript.reasoning_delta("private provider text".into());
        transcript.tool_started(
            "t".into(),
            ToolKind::NamedTask {
                name: "test".into(),
            },
        );
        transcript.tool_completed("t", Ok(code.clone()));
        transcript.provisional("p".into(), "main.go".into(), None, code);
        let copy = transcript.copy_text();
        assert!(copy.contains("You:\nshow the Go code"));
        assert!(copy.contains("line_19"));
        assert!(copy.contains("    + line_19"));
        assert!(!copy.contains("private provider text"));
        assert!(transcript.copy_entry(1).unwrap().contains("line_19\n```"));
        assert!(transcript.copy_entry(4).unwrap().contains("line_19"));
        transcript.toggle_reasoning();
        assert!(transcript.copy_text().contains("private provider text"));
    }

    #[test]
    fn multiple_writes_to_same_path_keep_distinct_commit_states() {
        let mut transcript = Transcript::default();
        transcript.provisional("a".into(), "game.lua".into(), None, "old".into());
        transcript.provisional(
            "b".into(),
            "game.lua".into(),
            Some("old".into()),
            "new".into(),
        );
        transcript.committed("a", "game.lua");
        let states: Vec<bool> = transcript
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Preview(preview) => Some(preview.committed),
                _ => None,
            })
            .collect();
        assert_eq!(states, [true, false]);
    }
}
