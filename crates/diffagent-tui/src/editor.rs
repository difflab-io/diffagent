use crate::sanitize::visible;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;

const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Default, Debug)]
pub struct Editor {
    text: String,
    /// Grapheme index, never a byte or scalar index.
    cursor: usize,
}

impl Editor {
    fn byte_index(&self) -> usize {
        self.text
            .grapheme_indices(true)
            .nth(self.cursor)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Paste keeps newlines but renders control characters safely; it never submits.
    pub fn paste(&mut self, text: &str) -> bool {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let text = text.split('\n').map(visible).collect::<Vec<_>>().join("\n");
        if self.text.len().saturating_add(text.len()) > MAX_MESSAGE_BYTES {
            return false;
        }
        let index = self.byte_index();
        self.text.insert_str(index, &text);
        self.cursor = self.text[..index + text.len()].graphemes(true).count();
        true
    }

    pub fn handle(&mut self, key: KeyEvent) -> Option<String> {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, modifiers) if modifiers.contains(KeyModifiers::SHIFT) => {
                self.paste("\n");
            }
            (KeyCode::Enter, _) => {
                let message = std::mem::take(&mut self.text);
                self.cursor = 0;
                return (!message.trim().is_empty()).then_some(message);
            }
            (KeyCode::Left, _) => self.cursor = self.cursor.saturating_sub(1),
            (KeyCode::Right, _) => {
                self.cursor = (self.cursor + 1).min(self.text.graphemes(true).count())
            }
            (KeyCode::Home, _) => self.cursor = 0,
            (KeyCode::End, _) => self.cursor = self.text.graphemes(true).count(),
            (KeyCode::Backspace, _) if self.cursor > 0 => {
                let end = self.byte_index();
                self.cursor -= 1;
                self.text.replace_range(self.byte_index()..end, "");
            }
            (KeyCode::Delete, _) => {
                let start = self.byte_index();
                let end = self
                    .text
                    .grapheme_indices(true)
                    .nth(self.cursor + 1)
                    .map_or(self.text.len(), |(i, _)| i);
                self.text.replace_range(start..end, "");
            }
            (KeyCode::Char(c), modifiers)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let mut buffer = [0; 4];
                self.paste(c.encode_utf8(&mut buffer));
            }
            _ => {}
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let prefix = &self.text[..self.byte_index()];
        let width = usize::from(area.width.saturating_sub(2).max(1));
        let height = usize::from(area.height.saturating_sub(2).max(1));
        let rows: usize = prefix
            .split('\n')
            .map(|line| {
                let columns = ratatui::text::Line::from(line).width();
                1 + columns.saturating_sub(1) / width
            })
            .sum();
        let scroll = rows.saturating_sub(height).min(u16::MAX as usize) as u16;
        frame.render_widget(
            Paragraph::new(self.text.as_str())
                .block(
                    Block::default()
                        .title("Message (Enter sends, Shift-Enter adds line)")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0))
                .style(Style::default()),
            area,
        );
        let last_line = prefix.rsplit('\n').next().unwrap_or_default();
        let x = ratatui::text::Line::from(last_line).width() % width;
        frame.set_cursor_position((
            area.x.saturating_add(1).saturating_add(x as u16),
            area.y
                .saturating_add(1)
                .saturating_add((rows - 1).saturating_sub(scroll as usize) as u16),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletes_graphemes_not_bytes() {
        let mut e = Editor::default();
        for c in "a👩\u{200d}💻".chars() {
            e.handle(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        e.handle(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(e.text, "a");
    }

    #[test]
    fn paste_keeps_line_breaks_without_terminal_control_bytes() {
        let mut editor = Editor::default();
        assert!(editor.paste("before\x1b[2J\tafter\nnext"));
        assert_eq!(editor.text, "before�[2J    after\nnext");
    }

    #[test]
    fn bracketed_paste_preserves_multiline_text_until_enter() {
        let mut editor = Editor::default();
        assert!(editor.paste("first\r\nsecond 👋"));
        editor.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        assert!(editor.paste("third"));
        assert_eq!(
            editor
                .handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .unwrap(),
            "first\nsecond 👋\nthird"
        );
        assert_eq!(editor.text, "");
        assert!(!editor.paste(&"x".repeat(MAX_MESSAGE_BYTES + 1)));
    }
}
