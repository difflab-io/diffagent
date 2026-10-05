//! A small CSV record parser.
//!
//! Parses exactly one CSV record (no multi-line support).
//!
//! Rules:
//! - Commas separate fields.
//! - A field may be quoted only from its first character.
//! - Commas inside quotes are literal.
//! - `""` inside a quoted field means one literal `"`.
//! - Whitespace is data and is never trimmed.
//! - Empty input is one empty field; a trailing comma adds an empty field.

/// Parse a single CSV record into its fields.
///
/// Returns an error `String` describing the problem when the input is not a
/// valid single CSV record.
pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String> {
    #[derive(Clone, Copy)]
    enum State {
        /// At the beginning of a field (nothing consumed for it yet).
        FieldStart,
        /// Inside an unquoted field.
        Unquoted,
        /// Inside a quoted field (opening quote consumed).
        Quoted,
        /// Just saw the closing quote of a quoted field.
        AfterQuote,
    }

    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut state = State::FieldStart;

    loop {
        match state {
            State::FieldStart => match chars.peek() {
                // End of input with no field content yet: the final (empty)
                // field. This also covers the entirely empty input case.
                None => {
                    fields.push(current);
                    break;
                }
                Some(&'"') => {
                    // A field may be quoted only from its first character.
                    chars.next();
                    state = State::Quoted;
                }
                Some(&',') => {
                    chars.next();
                    fields.push(std::mem::take(&mut current));
                    // Stay in FieldStart for the next field.
                }
                Some(_) => {
                    state = State::Unquoted;
                }
            },
            State::Unquoted => match chars.next() {
                None => {
                    fields.push(current);
                    break;
                }
                Some(',') => {
                    fields.push(std::mem::take(&mut current));
                    state = State::FieldStart;
                }
                Some('"') => {
                    return Err(
                        "unexpected quote inside an unquoted field".to_string()
                    );
                }
                Some(c) => current.push(c),
            },
            State::Quoted => match chars.next() {
                None => return Err("unclosed quoted field".to_string()),
                Some('"') => {
                    if let Some(&'"') = chars.peek() {
                        // `""` inside a quoted field: one literal quote.
                        chars.next();
                        current.push('"');
                    } else {
                        state = State::AfterQuote;
                    }
                }
                Some(c) => current.push(c),
            },
            State::AfterQuote => match chars.peek() {
                None => {
                    fields.push(current);
                    break;
                }
                Some(&',') => {
                    chars.next();
                    fields.push(std::mem::take(&mut current));
                    state = State::FieldStart;
                }
                Some(_) => {
                    return Err(
                        "unexpected characters after closing quote".to_string()
                    );
                }
            },
        }
    }

    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::parse_csv_line;

    fn ok(input: &str, expected: &[&str]) {
        let got = parse_csv_line(input).unwrap_or_else(|e| {
            panic!("expected Ok for {:?}, got Err({:?})", input, e)
        });
        let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(got, expected, "input = {:?}", input);
    }

    #[test]
    fn basic_fields() {
        ok("a,b,c", &["a", "b", "c"]);
    }

    #[test]
    fn single_field() {
        ok("abc", &["abc"]);
    }

    #[test]
    fn empty_input_is_one_empty_field() {
        ok("", &[""]);
    }

    #[test]
    fn trailing_comma_adds_empty_field() {
        ok("a,", &["a", ""]);
    }

    #[test]
    fn leading_comma_adds_empty_field() {
        ok(",a", &["", "a"]);
    }

    #[test]
    fn consecutive_commas_are_empty_fields() {
        ok("a,,b", &["a", "", "b"]);
    }

    #[test]
    fn only_commas() {
        ok(",,", &["", "", ""]);
    }

    #[test]
    fn quoted_field_with_comma() {
        ok("a,\"b,c\",d", &["a", "b,c", "d"]);
    }

    #[test]
    fn quoted_field_whole_input() {
        ok("\"hello world\"", &["hello world"]);
    }

    #[test]
    fn escaped_quotes_become_one_quote() {
        // "a""b" -> a"b
        ok("\"a\"\"b\"", &["a\"b"]);
    }

    #[test]
    fn quoted_empty_field() {
        ok("\"\"", &[""]);
    }

    #[test]
    fn quoted_field_of_only_escaped_quote_pair() {
        // """" -> one literal quote
        ok("\"\"\"\"", &["\""]);
    }

    #[test]
    fn quoted_comma_only() {
        ok("\",\"", &[","]);
    }

    #[test]
    fn whitespace_is_data() {
        ok(" a , b ", &[" a ", " b "]);
    }

    #[test]
    fn whitespace_inside_quotes_is_preserved() {
        ok("\"  a  \"", &["  a  "]);
    }

    #[test]
    fn quoted_field_after_comma() {
        ok("\"x\",\"y\"", &["x", "y"]);
    }

    #[test]
    fn reject_unclosed_quote() {
        assert!(parse_csv_line("\"abc").is_err());
    }

    #[test]
    fn reject_unclosed_quote_with_comma() {
        assert!(parse_csv_line("a,\"b,c").is_err());
    }

    #[test]
    fn reject_quote_inside_unquoted_field() {
        assert!(parse_csv_line("ab\"c").is_err());
    }

    #[test]
    fn reject_quote_in_unquoted_field_after_comma() {
        assert!(parse_csv_line("a,b\"c").is_err());
    }

    #[test]
    fn reject_chars_after_closing_quote() {
        assert!(parse_csv_line("\"a\"b").is_err());
    }

    #[test]
    fn reject_space_after_closing_quote() {
        // Whitespace is data, so a trailing space after the closing quote is
        // an unexpected character.
        assert!(parse_csv_line("\"a\" ").is_err());
    }

    #[test]
    fn reject_quote_after_closing_quote_without_comma() {
        // "a"b -> b after closing quote
        assert!(parse_csv_line("\"a\"\"b\"").is_ok()); // sanity: this is a"b"
        assert!(parse_csv_line("\"a\"\"b").is_err()); // unclosed at end
        assert!(parse_csv_line("\"a\"b\"").is_err()); // char after closing quote
    }
}
