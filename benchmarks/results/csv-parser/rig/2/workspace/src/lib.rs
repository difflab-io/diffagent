//! A tiny single-record CSV parser.
//!
//! Rules implemented (single line only, no multi-line records):
//! - Commas separate fields.
//! - A field may be quoted only from its very first character.
//! - Commas inside a quoted field are literal data.
//! - `""` inside a quoted field represents a single literal `"`.
//! - An unclosed quote is an error.
//! - A `"` inside an unquoted field is an error.
//! - Any character after a closing quote other than a comma is an error.
//! - Whitespace is significant; nothing is trimmed.
//! - Empty input yields a single empty field; a trailing comma adds an empty
//!   field.

/// Parser state for the field currently being read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// At the beginning of a field; the next character decides how it is read.
    Start,
    /// Inside an unquoted field.
    Unquoted,
    /// Inside a quoted field (opening quote consumed).
    Quoted,
    /// Just consumed the closing quote of a quoted field.
    AfterQuote,
}

/// Parse a single CSV record from `input`.
///
/// Returns the parsed fields on success, or a human-readable error message on
/// the first rule violation encountered.
pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut state = State::Start;
    let mut chars = input.chars().peekable();

    loop {
        match chars.next() {
            None => {
                if state == State::Quoted {
                    return Err("unclosed quote: quoted field is never closed".to_string());
                }
                fields.push(current);
                break;
            }
            Some(c) => match state {
                State::Start => {
                    if c == '"' {
                        // Quoted field begins only if the quote is the first char.
                        state = State::Quoted;
                    } else if c == ',' {
                        // Empty field before the comma; stay at the start of a
                        // new field.
                        fields.push(String::new());
                    } else {
                        current.push(c);
                        state = State::Unquoted;
                    }
                }
                State::Unquoted => {
                    if c == ',' {
                        fields.push(std::mem::take(&mut current));
                        state = State::Start;
                    } else if c == '"' {
                        return Err(
                            "quote inside unquoted field: fields may only be quoted from the \
                             first character"
                                .to_string(),
                        );
                    } else {
                        current.push(c);
                    }
                }
                State::Quoted => {
                    if c == '"' {
                        // Either an escaped quote (`""`) or the closing quote.
                        if chars.peek() == Some(&'"') {
                            chars.next();
                            current.push('"');
                        } else {
                            state = State::AfterQuote;
                        }
                    } else {
                        current.push(c);
                    }
                }
                State::AfterQuote => {
                    if c == ',' {
                        fields.push(std::mem::take(&mut current));
                        state = State::Start;
                    } else {
                        return Err(format!(
                            "unexpected character `{c}` after closing quote: only a comma may \
                             follow a quoted field"
                        ));
                    }
                }
            },
        }
    }

    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(input: &str) -> Vec<String> {
        parse_csv_line(input).expect("expected successful parse")
    }

    fn err(input: &str) -> String {
        parse_csv_line(input).expect_err("expected parse error")
    }

    #[test]
    fn simple_fields() {
        assert_eq!(ok("a,b,c"), vec!["a", "b", "c"]);
        assert_eq!(ok("single"), vec!["single"]);
    }

    #[test]
    fn empty_input_is_one_empty_field() {
        assert_eq!(ok(""), vec![""]);
    }

    #[test]
    fn commas_yield_empty_fields() {
        assert_eq!(ok(","), vec!["", ""]);
        assert_eq!(ok("a,,b"), vec!["a", "", "b"]);
        assert_eq!(ok(",,"), vec!["", "", ""]);
    }

    #[test]
    fn trailing_comma_adds_empty_field() {
        assert_eq!(ok("a,"), vec!["a", ""]);
        assert_eq!(ok("a,b,"), vec!["a", "b", ""]);
    }

    #[test]
    fn quoted_field_with_commas() {
        assert_eq!(ok("\"a,b\",c"), vec!["a,b", "c"]);
        assert_eq!(ok("\"a,b,c\""), vec!["a,b,c"]);
        assert_eq!(ok("x,\"a,b\""), vec!["x", "a,b"]);
    }

    #[test]
    fn quoted_empty_field() {
        assert_eq!(ok("\"\""), vec![""]);
        assert_eq!(ok("\"\",\"\""), vec!["", ""]);
    }

    #[test]
    fn doubled_quote_is_literal_quote() {
        assert_eq!(ok("\"a\"\"b\""), vec!["a\"b"]);
        assert_eq!(ok("\"\"\"\""), vec!["\""]);
        assert_eq!(ok("\"say \"\"hi\"\"\""), vec!["say \"hi\""]);
    }

    #[test]
    fn whitespace_is_data_not_trimmed() {
        assert_eq!(ok(" a , b "), vec![" a ", " b "]);
        assert_eq!(ok("\t,\n"), vec!["\t", "\n"]);
        assert_eq!(ok("\" a \""), vec![" a "]);
    }

    #[test]
    fn unclosed_quote_is_error() {
        assert!(parse_csv_line("\"abc").is_err());
        assert!(parse_csv_line("a,\"b").is_err());
    }

    #[test]
    fn quote_inside_unquoted_field_is_error() {
        assert!(parse_csv_line("ab\"c").is_err());
        assert!(parse_csv_line("a\"b\"").is_err());
        // Whitespace before a quote means it is not the first character.
        assert!(parse_csv_line("  \"a\"").is_err());
    }

    #[test]
    fn characters_after_closing_quote_are_error() {
        assert!(parse_csv_line("\"a\"b").is_err());
        assert!(parse_csv_line("\"a\" ").is_err());
        // An odd number of quotes leaves an open quoted field.
        assert!(parse_csv_line("\"a\"\"").is_err());
    }

    #[test]
    fn error_messages_are_nonempty() {
        assert!(!err("\"abc").is_empty());
        assert!(!err("ab\"c").is_empty());
        assert!(!err("\"a\"b").is_empty());
    }

    #[test]
    fn quoted_field_ending_line_is_ok() {
        assert_eq!(ok("\"a\""), vec!["a"]);
        assert_eq!(ok("a,\"b\""), vec!["a", "b"]);
    }
}
