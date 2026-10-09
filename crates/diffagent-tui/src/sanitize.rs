/// Preserve line breaks for clipboard text, but never copy terminal control bytes.
pub(crate) fn copyable(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '\n' | '\t' => ch,
            ch if ch.is_control() => '�',
            ch => ch,
        })
        .collect()
}

/// Never send model, tool, or workspace control bytes to the terminal.
/// Keep their presence visible rather than silently hiding them.
pub(crate) fn visible(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\t' => output.push_str("    "),
            ch if ch.is_control() => output.push('�'),
            ch => output.push(ch),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_sequences_cannot_escape_the_transcript() {
        assert_eq!(visible("before\x1b[2J\r\tafter"), "before�[2J�    after");
        assert_eq!(copyable("a\n\x1b[2J\tb"), "a\n�[2J\tb");
    }
}
