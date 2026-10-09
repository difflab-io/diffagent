use crate::sanitize::visible;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Read {
        path: String,
    },
    Write {
        path: String,
    },
    /// A task name is descriptive text, never a shell command.
    NamedTask {
        name: String,
    },
    Other {
        name: String,
        detail: String,
    },
}

impl ToolKind {
    fn label(&self) -> String {
        match self {
            Self::Read { path } => format!("read {path}"),
            Self::Write { path } => format!("write {path}"),
            Self::NamedTask { name } => format!("task {name}"),
            Self::Other { name, detail } if detail.is_empty() => name.clone(),
            Self::Other { name, detail } => format!("{name} {detail}"),
        }
    }
}

#[derive(Debug)]
pub struct ToolItem {
    pub id: String,
    pub kind: ToolKind,
    pub result: Option<Result<String, String>>,
}

impl ToolItem {
    pub fn started_line(&self) -> Line<'static> {
        let (state, color) = if self.result.is_none() {
            ("running", Color::Yellow)
        } else {
            ("started", Color::Cyan)
        };
        Line::from(vec![
            Span::styled(
                format!("  [{state}] "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                visible(&self.kind.label()),
                Style::default().fg(Color::Cyan),
            ),
        ])
    }

    pub fn lines(&self) -> Vec<Line<'static>> {
        self.render_lines(Some(12))
    }

    pub fn full_lines(&self) -> Vec<Line<'static>> {
        self.render_lines(None)
    }

    fn render_lines(&self, cap: Option<usize>) -> Vec<Line<'static>> {
        let mut lines = vec![self.line()];
        if let Some(result) = &self.result {
            let (output, color) = match result {
                Ok(output) => (output, Color::Gray),
                Err(output) => (output, Color::Red),
            };
            let mut parts = output.lines();
            for part in parts.by_ref().take(cap.unwrap_or(usize::MAX)) {
                let mut chars = part.chars();
                let snippet = chars
                    .by_ref()
                    .take(if cap.is_some() { 240 } else { usize::MAX })
                    .collect::<String>();
                let suffix = if chars.next().is_some() { " …" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}{suffix}", visible(&snippet)),
                        Style::default().fg(color),
                    ),
                ]));
            }
            let omitted = parts.count();
            if omitted > 0 {
                lines.push(Line::styled(
                    format!("    │ … {omitted} more lines"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
        lines
    }

    pub fn new(id: String, kind: ToolKind) -> Self {
        Self {
            id,
            kind,
            result: None,
        }
    }

    pub fn line(&self) -> Line<'static> {
        let (state, color) = match &self.result {
            None => ("running", Color::Yellow),
            Some(Ok(_)) => ("done", Color::Green),
            Some(Err(_)) => ("failed", Color::Red),
        };
        let mut spans = vec![
            Span::styled(
                format!("  [{state}] "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                visible(&self.kind.label()),
                Style::default().fg(Color::Cyan),
            ),
        ];
        if let Some(result) = &self.result {
            let summary = match (&self.kind, result) {
                (ToolKind::Read { .. }, Ok(_)) => "read complete".to_owned(),
                (_, Ok(output) | Err(output)) => visible(
                    output
                        .lines()
                        .find(|line| !line.trim().is_empty())
                        .unwrap_or_default(),
                )
                .chars()
                .take(100)
                .collect(),
            };
            if !summary.is_empty() {
                spans.push(Span::styled(
                    format!(" — {summary}"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
        Line::from(spans)
    }
}
