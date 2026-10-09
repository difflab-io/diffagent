use crate::sanitize::visible;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// A proposal in the chat timeline. A later commit is rendered separately.
/// Rendering this value never writes a file.
#[derive(Debug)]
pub struct WritePreview {
    pub id: String,
    pub path: String,
    original: Option<String>,
    content: String,
    pub committed: bool,
}

impl WritePreview {
    pub fn new(id: String, path: String, original: Option<String>, content: String) -> Self {
        Self {
            id,
            path,
            original,
            content,
            committed: false,
        }
    }

    pub fn lines(&self) -> Vec<Line<'static>> {
        let mut lines = self.full_lines();
        if lines.len() > 13 {
            let hidden = lines.len() - 13;
            lines.truncate(13);
            lines.push(Line::styled(
                format!("      … {hidden} more lines · click or Ctrl+O to expand"),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines
    }

    pub fn full_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from(vec![
            Span::styled("    ↳ ", Style::default().fg(Color::DarkGray)),
            Span::styled(visible(&self.path), Style::default().fg(Color::Cyan)),
            Span::styled(
                "  provisional · not yet written",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])];
        let lang = self.path.rsplit('.').next().unwrap_or_default();
        lines.extend(diff_lines(self.original.as_deref(), &self.content, lang));
        lines
    }
}

/// Show the actual changed region with a little context, rather than dumping
/// the whole original file into the conversation. Long changes stay bounded.
fn diff_lines(original: Option<&str>, content: &str, lang: &str) -> Vec<Line<'static>> {
    let old: Vec<&str> = original.unwrap_or_default().lines().collect();
    let new: Vec<&str> = content.lines().collect();
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let mut lines = Vec::new();
    if prefix > 2 {
        lines.push(omitted(prefix - 2));
    }
    for line in &new[prefix.saturating_sub(2)..prefix] {
        lines.push(marked(line, lang, "      ", Color::DarkGray));
    }
    for line in &old[prefix..old.len() - suffix] {
        lines.push(Line::styled(
            format!("    - {}", visible(line)),
            Style::default().fg(Color::Red),
        ));
    }
    for line in &new[prefix..new.len() - suffix] {
        lines.push(marked(line, lang, "    + ", Color::Green));
    }
    for line in &new[new.len() - suffix..(new.len() - suffix + suffix.min(2))] {
        lines.push(marked(line, lang, "      ", Color::DarkGray));
    }
    if suffix > 2 {
        lines.push(omitted(suffix - 2));
    }
    lines
}

fn omitted(count: usize) -> Line<'static> {
    Line::styled(
        format!("      … {count} unchanged or additional lines"),
        Style::default().fg(Color::DarkGray),
    )
}

fn marked(line: &str, lang: &str, marker: &str, color: Color) -> Line<'static> {
    let mut spans = vec![Span::styled(marker.to_owned(), Style::default().fg(color))];
    let highlighted = highlight(&visible(line), lang);
    spans.extend(highlighted.spans.into_iter().map(|span| {
        Span::styled(
            span.content.into_owned(),
            highlighted.style.patch(span.style),
        )
    }));
    Line::from(spans)
}

/// Small, non-executing lexical highlighter for fenced code and file diffs.
/// Unknown languages are shown as plain text.
pub(crate) fn highlight(line: &str, lang: &str) -> Line<'static> {
    if lang == "diff" {
        let color = match line.chars().next() {
            Some('+') => Color::Green,
            Some('-') => Color::Red,
            Some('@') => Color::Yellow,
            _ => Color::DarkGray,
        };
        return Line::styled(line.to_owned(), Style::default().fg(color));
    }
    let comment = match lang {
        "rs" | "rust" | "js" | "jsx" | "javascript" | "ts" | "tsx" | "typescript" | "go"
        | "java" | "c" | "cpp" | "swift" => "//",
        "lua" => "--",
        "py" | "python" | "sh" | "shell" | "bash" | "ruby" | "toml" | "yaml" | "yml" => "#",
        "json" | "sql" | "html" | "css" => "",
        _ => return Line::raw(line.to_owned()),
    };
    let mut spans = Vec::new();
    let mut iter = line.char_indices().peekable();
    while let Some((start, c)) = iter.next() {
        if !comment.is_empty() && line[start..].starts_with(comment) {
            spans.push(Span::styled(
                line[start..].to_owned(),
                Style::default().fg(Color::DarkGray),
            ));
            break;
        }
        let end = if c.is_alphabetic() || c == '_' {
            while let Some(&(_, next)) = iter.peek() {
                if next.is_alphanumeric() || next == '_' {
                    iter.next();
                } else {
                    break;
                }
            }
            iter.peek().map_or(line.len(), |(i, _)| *i)
        } else if c == '"' || c == '\'' || c == '`' {
            let mut escaped = false;
            for (_, next) in iter.by_ref() {
                if next == c && !escaped {
                    break;
                }
                if next == '\\' {
                    escaped = !escaped;
                } else {
                    escaped = false;
                }
            }
            iter.peek().map_or(line.len(), |(i, _)| *i)
        } else {
            start + c.len_utf8()
        };
        let word = &line[start..end];
        let color = if word.starts_with(['"', '\'', '`']) {
            Color::Yellow
        } else if word.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            Color::LightCyan
        } else if matches!(
            word,
            "fn" | "let"
                | "pub"
                | "struct"
                | "enum"
                | "impl"
                | "use"
                | "match"
                | "async"
                | "await"
                | "return"
                | "local"
                | "function"
                | "end"
                | "if"
                | "then"
                | "else"
                | "for"
                | "while"
                | "const"
                | "class"
                | "def"
                | "import"
                | "from"
                | "true"
                | "false"
                | "null"
                | "nil"
        ) {
            Color::Magenta
        } else {
            Color::Reset
        };
        spans.push(Span::styled(word.to_owned(), Style::default().fg(color)));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_keeps_context_and_marks_replacement() {
        let lines = diff_lines(Some("same\nold\nend"), "same\nnew\nend", "rs");
        let rendered: Vec<String> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        assert_eq!(
            rendered,
            ["      same", "    - old", "    + new", "      end"]
        );
    }

    #[test]
    fn preview_keeps_control_bytes_out_of_terminal_spans() {
        let lines = diff_lines(None, "fn demo() {\x1b[2J}\n", "rs");
        let text = lines
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(!text.contains('\x1b'));
        assert!(text.contains("�[2J"));
    }

    #[test]
    fn code_keywords_and_strings_are_colored() {
        let line = highlight("fn demo() { return \"ok\"; }", "rust");
        assert_eq!(line.spans[0].style.fg, Some(Color::Magenta));
        assert!(
            line.spans
                .iter()
                .any(|span| span.content == "\"ok\"" && span.style.fg == Some(Color::Yellow))
        );
    }
}
