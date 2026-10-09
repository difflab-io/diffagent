//! CommonMark text rendered as terminal spans, without changing font size.
//! Fenced code uses the same small lexical highlighter as write diffs.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{sanitize::visible, write_preview::highlight};

/// Keep the transcript compact while retaining the full source for a live modal.
pub fn render_inline(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut code_lines = 0;
    let mut hidden = 0;
    let mut in_code = false;
    for line in render(text) {
        let marker = line
            .spans
            .first()
            .map(|span| span.content.as_ref())
            .unwrap_or("");
        if marker.starts_with("    ┌─ ") {
            in_code = true;
            code_lines = 0;
            hidden = 0;
        } else if marker == "    └─" {
            if hidden > 0 {
                lines.push(Line::styled(
                    format!("    … {hidden} more lines · click or Ctrl+O to expand"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            in_code = false;
        } else if in_code {
            code_lines += 1;
            if code_lines > 12 {
                hidden += 1;
                continue;
            }
        }
        lines.push(line);
    }
    if hidden > 0 && in_code {
        lines.push(Line::styled(
            format!("    … {hidden} more lines · click or Ctrl+O to expand"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines
}

pub fn render(text: &str) -> Vec<Line<'static>> {
    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let mut view = Markdown::default();
    for event in Parser::new_ext(text, options) {
        view.event(event);
    }
    if !view.current.spans.is_empty() {
        view.new_line();
    }
    while view.lines.last().is_some_and(|line| line.spans.is_empty()) {
        view.lines.pop();
    }
    view.lines
}

#[derive(Default)]
struct Markdown {
    lines: Vec<Line<'static>>,
    current: Line<'static>,
    styles: Vec<Style>,
    code_lang: Option<String>,
    lists: Vec<Option<u64>>,
    quote_depth: usize,
    links: Vec<String>,
}

impl Markdown {
    fn style(&self) -> Style {
        self.styles
            .iter()
            .fold(Style::default(), |style, next| style.patch(*next))
    }

    fn append(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        if self.current.spans.is_empty() && self.quote_depth > 0 {
            self.current.spans.push(Span::styled(
                "│ ".repeat(self.quote_depth),
                Style::default().fg(Color::DarkGray),
            ));
        }
        self.current
            .spans
            .push(Span::styled(visible(text), self.style().patch(style)));
    }

    fn new_line(&mut self) {
        self.lines.push(std::mem::take(&mut self.current));
    }

    fn end_block(&mut self) {
        if !self.current.spans.is_empty() {
            self.new_line();
        }
        if self.lines.last().is_some_and(|line| !line.spans.is_empty()) {
            self.lines.push(Line::default());
        }
    }

    fn text(&mut self, text: &str) {
        for part in text.split_inclusive('\n') {
            let body = part.strip_suffix('\n').unwrap_or(part);
            if let Some(lang) = &self.code_lang {
                if self.current.spans.is_empty() {
                    self.current
                        .spans
                        .push(Span::styled("    │ ", Style::default().fg(Color::DarkGray)));
                }
                let highlighted = highlight(&visible(body), lang);
                let base = self.style().patch(highlighted.style);
                for span in highlighted.spans {
                    self.current.spans.push(Span::styled(
                        span.content.into_owned(),
                        base.patch(span.style),
                    ));
                }
            } else {
                self.append(body, Style::default());
            }
            if part.ends_with('\n') {
                self.new_line();
            }
        }
    }

    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                self.end_block();
                let color = match level as u8 {
                    1 => Color::LightCyan,
                    2 => Color::Cyan,
                    _ => Color::LightBlue,
                };
                self.styles
                    .push(Style::default().fg(color).add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Heading(_)) => {
                self.styles.pop();
                self.end_block();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                self.end_block();
                let lang = match kind {
                    CodeBlockKind::Fenced(label) => label
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase(),
                    CodeBlockKind::Indented => String::new(),
                };
                let lang = visible(&lang);
                self.lines.push(Line::styled(
                    format!("    ┌─ {}", if lang.is_empty() { "code" } else { &lang }),
                    Style::default().fg(Color::DarkGray),
                ));
                self.code_lang = Some(lang);
            }
            Event::End(TagEnd::CodeBlock) => {
                if !self.current.spans.is_empty() {
                    self.new_line();
                }
                self.code_lang = None;
                self.lines
                    .push(Line::styled("    └─", Style::default().fg(Color::DarkGray)));
                self.end_block();
            }
            Event::Start(Tag::Strong) => self
                .styles
                .push(Style::default().add_modifier(Modifier::BOLD)),
            Event::End(TagEnd::Strong) => {
                self.styles.pop();
            }
            Event::Start(Tag::Emphasis) => self
                .styles
                .push(Style::default().add_modifier(Modifier::ITALIC)),
            Event::End(TagEnd::Emphasis) => {
                self.styles.pop();
            }
            Event::Start(Tag::Strikethrough) => self
                .styles
                .push(Style::default().add_modifier(Modifier::CROSSED_OUT)),
            Event::End(TagEnd::Strikethrough) => {
                self.styles.pop();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                self.links.push(dest_url.to_string());
                self.styles.push(
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::UNDERLINED),
                );
            }
            Event::End(TagEnd::Link) => {
                self.styles.pop();
                if let Some(url) = self.links.pop() {
                    let short: String = url.chars().take(120).collect();
                    self.append(&format!(" ({short})"), Style::default().fg(Color::DarkGray));
                }
            }
            Event::Start(Tag::List(start)) => {
                if !self.current.spans.is_empty() {
                    self.new_line();
                }
                self.lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                self.lists.pop();
                self.end_block();
            }
            Event::Start(Tag::Item) => {
                if !self.current.spans.is_empty() {
                    self.new_line();
                }
                let indent = "  ".repeat(self.lists.len().saturating_sub(1));
                let marker = match self.lists.last_mut() {
                    Some(Some(index)) => {
                        let label = format!("{index}. ");
                        *index += 1;
                        label
                    }
                    _ => "• ".to_owned(),
                };
                self.append(
                    &format!("  {indent}{marker}"),
                    Style::default().fg(Color::Cyan),
                );
            }
            Event::End(TagEnd::Item) => {
                if !self.current.spans.is_empty() {
                    self.new_line();
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                self.quote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                self.end_block();
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
            Event::End(TagEnd::Paragraph) => {
                if self.lists.is_empty() {
                    self.end_block();
                } else if !self.current.spans.is_empty() {
                    self.new_line();
                }
            }
            Event::Text(value) | Event::Html(value) | Event::InlineHtml(value) => {
                self.text(&value);
            }
            Event::Code(value) => self.append(
                &value,
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Rgb(35, 40, 50)),
            ),
            // Preserve the author's line breaks in a terminal conversation.
            Event::SoftBreak => self.new_line(),
            Event::HardBreak => self.new_line(),
            Event::Rule => {
                self.end_block();
                self.lines.push(Line::styled(
                    "  ────────────",
                    Style::default().fg(Color::DarkGray),
                ));
                self.end_block();
            }
            Event::TaskListMarker(done) => self.append(
                if done { "[x] " } else { "[ ] " },
                Style::default().fg(Color::Green),
            ),
            Event::End(TagEnd::TableRow) => self.new_line(),
            Event::End(TagEnd::TableCell) => {
                self.append(" │ ", Style::default().fg(Color::DarkGray))
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_is_color_not_font_size_or_markdown_syntax() {
        let lines = render("# Heading\n\nA **bold** word and `code`.");
        assert_eq!(lines[0].spans[0].content, "Heading");
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::LightCyan));
        assert!(
            lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
        assert!(
            lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| { span.content == "code" && span.style.fg == Some(Color::Yellow) })
        );
    }

    #[test]
    fn fenced_code_has_language_and_colored_keywords() {
        let lines = render("```rust\nfn hello() {}\n```");
        assert!(lines[0].spans[0].content.contains("rust"));
        assert!(
            lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| { span.content == "fn" && span.style.fg == Some(Color::Magenta) })
        );
        assert!(
            !lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.content.contains("```"))
        );
    }

    #[test]
    fn streamed_code_is_capped_inline_but_full_view_keeps_every_line() {
        let text = format!(
            "```rust\n{}\n```",
            (0..18)
                .map(|n| format!("let n{n} = {n};"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let inline = render_inline(&text)
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let full = render(&text)
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(inline.contains("6 more lines"));
        assert!(!inline.contains("n17"));
        assert!(full.contains("n17"));
    }

    #[test]
    fn javascript_and_diff_fences_use_syntax_colors() {
        let javascript = render("```javascript\nconst answer = 42;\n```");
        assert!(
            javascript
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| { span.content == "const" && span.style.fg == Some(Color::Magenta) })
        );
        let diff = render("```diff\n+ added\n- removed\n```");
        assert!(
            diff.iter()
                .flat_map(|line| &line.spans)
                .any(|span| { span.content == "+ added" && span.style.fg == Some(Color::Green) })
        );
    }

    #[test]
    fn literal_html_like_paths_remain_visible() {
        let lines = render("Please edit <file> now.");
        let text = lines
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(text.contains("<file>"));
    }

    #[test]
    fn fenced_code_never_emits_terminal_control_bytes() {
        let lines = render("```rust\nlet value = \"\x1b[2J\";\n```");
        let text = lines
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(!text.contains('\x1b'));
        assert!(text.contains("�[2J"));
    }

    #[test]
    fn links_lists_and_quotes_keep_readable_text() {
        let lines = render("> A quote\n\n- [link](https://example.test)\n- next");
        let text = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("│ A quote"));
        assert!(text.contains("• link (https://example.test)"));
        assert!(text.contains("• next"));
    }
}
