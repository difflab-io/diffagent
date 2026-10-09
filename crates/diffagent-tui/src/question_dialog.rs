use crate::sanitize::visible;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::cell::Cell;

/// `Some(Some(answer))` submits; `Some(None)` skips; `None` keeps editing.
pub struct QuestionDialog {
    pub id: String,
    question: String,
    options: Vec<String>,
    selected: usize,
    input: String,
    area: Cell<Rect>,
}

impl QuestionDialog {
    pub fn new(id: String, question: String, options: Vec<String>) -> Self {
        Self {
            id,
            question,
            options,
            selected: 0,
            input: String::new(),
            area: Cell::new(Rect::default()),
        }
    }

    pub fn paste(&mut self, text: &str) {
        let safe = visible(&text.replace(['\r', '\n'], " "));
        if self.input.len() + safe.len() <= 4096 {
            self.input.push_str(&safe);
        }
    }

    pub fn key(&mut self, key: KeyEvent) -> Option<Option<String>> {
        match key.code {
            KeyCode::Esc => Some(None),
            KeyCode::Enter if !self.input.trim().is_empty() => Some(Some(self.input.clone())),
            KeyCode::Enter if !self.options.is_empty() => {
                Some(Some(self.options[self.selected].clone()))
            }
            KeyCode::Enter => None,
            KeyCode::Up if !self.options.is_empty() => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Down if !self.options.is_empty() => {
                self.selected = (self.selected + 1).min(self.options.len() - 1);
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                None
            }
            KeyCode::Char(c @ '1'..='4')
                if key.modifiers.is_empty() && ((c as u8 - b'1') as usize) < self.options.len() =>
            {
                Some(Some(self.options[(c as u8 - b'1') as usize].clone()))
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL) && !c.is_control() =>
            {
                if self.input.len() + c.len_utf8() <= 4096 {
                    self.input.push(c);
                }
                None
            }
            _ => None,
        }
    }

    pub fn click(&self, x: u16, y: u16) -> Option<Option<String>> {
        let area = self.area.get();
        if x <= area.x
            || x >= area.right().saturating_sub(1)
            || y <= area.y
            || y >= area.bottom().saturating_sub(1)
        {
            return None;
        }
        let inner_width = area.width.saturating_sub(2).max(1) as usize;
        let question_rows = Line::from(visible(&self.question))
            .width()
            .max(1)
            .div_ceil(inner_width) as u16;
        let first_option = area.y.saturating_add(2).saturating_add(question_rows);
        if y < first_option {
            return None;
        }
        self.options
            .get((y - first_option) as usize)
            .cloned()
            .map(Some)
    }

    pub fn render(&self, frame: &mut Frame) {
        let screen = frame.area();
        let width = (screen.width.saturating_mul(4) / 5).max(1);
        let height = (screen.height.saturating_mul(3) / 5).max(1);
        let area = Rect::new(
            screen.x + (screen.width - width) / 2,
            screen.y + (screen.height - height) / 2,
            width,
            height,
        );
        self.area.set(area);
        let mut lines = vec![
            Line::styled(
                visible(&self.question),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::default(),
        ];
        for (index, option) in self.options.iter().enumerate() {
            let style = if index == self.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::styled(
                format!("  {}. {}", index + 1, visible(option)),
                style,
            ));
        }
        lines.push(Line::default());
        lines.push(Line::styled(
            format!("  Your answer: {}", self.input),
            Style::default().fg(Color::Green),
        ));
        lines.push(Line::styled(
            "  ↑/↓ select · 1–4 choose · Enter submit · Esc skip",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title("Agent asks you")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_an_option_uses_the_rendered_row() -> Result<(), std::convert::Infallible> {
        use ratatui::{Terminal, backend::TestBackend};
        let dialog =
            QuestionDialog::new("q".into(), "Choose".into(), vec!["Yes".into(), "No".into()]);
        let mut terminal = Terminal::new(TestBackend::new(80, 24))?;
        terminal.draw(|frame| dialog.render(frame))?;
        let area = dialog.area.get();
        assert_eq!(
            dialog.click(area.x + 4, area.y + 3),
            Some(Some("Yes".into()))
        );
        assert_eq!(
            dialog.click(area.x + 4, area.y + 4),
            Some(Some("No".into()))
        );
        assert_eq!(dialog.click(area.x + 4, area.y + 1), None);
        Ok(())
    }

    #[test]
    fn options_custom_answers_and_skips() {
        let mut dialog =
            QuestionDialog::new("q".into(), "Choose".into(), vec!["Yes".into(), "No".into()]);
        assert_eq!(
            dialog.key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            Some(Some("No".into()))
        );
        dialog.paste("something\x1b[2J");
        assert_eq!(
            dialog.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Some("something�[2J".into()))
        );
        assert_eq!(
            dialog.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(None)
        );
    }
}
