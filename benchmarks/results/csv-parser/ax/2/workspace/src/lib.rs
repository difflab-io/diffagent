//! A tiny single-record CSV parser.
//!
//! Parses exactly one CSV record (no multi-line records). Commas separate
//! fields. A field may be quoted only from its very first character. Inside a
//! quoted field, commas are literal and a doubled quote (`""`) represents one
//! literal `"`. Whitespace is preserved as data (no trimming).

/// Parse a single CSV record.
///
/// Rules:
/// - Commas separate fields.
/// - A field may be quoted only from its first character.
/// - Commas inside quotes are literal.
/// - `""` inside a quoted field means one literal `"`.
/// - An unclosed quote is an error.
/// - A quote inside an unquoted field is an error.
/// - Any character after a closing quote other than a comma is an error.
/// - Empty input means one empty field; a trailing comma adds an empty field.
pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String> {
    #[derive(PartialEq)]
    enum State {
        /// At the beginning of a field (nothing consumed yet).
        Start,
        /// Inside an unquoted field.
        Unquoted,
        /// Inside a quoted field.
        Quoted,
        /// Immediately after a field's closing quote.
        AfterQuote,
    }

    let chars: Vec<char> = input.chars().collect();
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut state = State::Start;

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match state {
            State::Start => match c {
                '"' => state = State::Quoted,
                ',' => fields.push(std::mem::take(&mut current)),
                _ => {
                    current.push(c);
                    state = State::Unquoted;
                }
            },
            State::Unquoted => match c {
                ',' => {
                    fields.push(std::mem::take(&mut current));
                    state = State::Start;
                }
                '"' => {
                    return Err(format!(
                        "unexpected quote inside unquoted field at position {}",
                        i
                    ));
                }
                _ => current.push(c),
            },
            State::Quoted => match c {
                '"' => {
                    if i + 1 < chars.len() && chars[i + 1] == '"' {
                        // Escaped quote: `""` -> one literal `"`.
                        current.push('"');
                        i += 1; // consume the second quote of the pair
                    } else {
                        state = State::AfterQuote;
                    }
                }
                _ => current.push(c),
            },
            State::AfterQuote => match c {
                ',' => {
                    fields.push(std::mem::take(&mut current));
                    state = State::Start;
                }
                _ => {
                    return Err(format!(
                        "unexpected character after closing quote at position {}",
                        i
                    ));
                }
            },
        }
        i += 1;
    }

    if state == State::Quoted {
        return Err("unclosed quote".to_string());
    }

    // Finish the last (possibly empty) field.
    fields.push(current);
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::parse_csv_line;

    fn ok(input: &str) -> Vec<String> {
        parse_csv_line(input).expect("expected Ok, got Err")
    }

    #[test]
    fn empty_input_is_one_empty_field() {
        assert_eq!(ok(""), vec!["".to_string()]);
    }

    #[test]
    fn simple_fields() {
        assert_eq!(ok("a,b,c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn trailing_comma_adds_empty_field() {
        assert_eq!(ok("a,"), vec!["a", ""]);
        assert_eq!(ok(","), vec!["", ""]);
        assert_eq!(ok("a,,b"), vec!["a", "", "b"]);
    }

    #[test]
    fn quoted_comma_is_literal() {
        assert_eq!(ok("\"a,b\",c"), vec!["a,b", "c"]);
    }

    #[test]
    fn escaped_quotes_inside_quoted_field() {
        assert_eq!(ok("\"a\"\"b\""), vec!["a\"b"]);
        assert_eq!(ok("\"\"\"\""), vec!["\""]);
        assert_eq!(ok("\"\""), vec![""]);
    }

    #[test]
    fn quote_may_start_a_field_only() {
        assert_eq!(ok("a,\"b\""), vec!["a", "b"]);
        assert_eq!(ok("\"\",x"), vec!["", "x"]);
    }

    #[test]
    fn whitespace_is_preserved() {
        assert_eq!(ok(" a , b "), vec![" a ", " b "]);
        assert_eq!(ok("\" a \""), vec![" a "]);
        assert_eq!(ok("  "), vec!["  "]);
    }

    #[test]
    fn quoted_field_may_contain_whitespace_and_commas() {
        assert_eq!(ok("\" x, y \",z"), vec![" x, y ", "z"]);
    }

    #[test]
    fn unclosed_quote_is_error() {
        assert!(parse_csv_line("\"abc").is_err());
        assert!(parse_csv_line("a,\"b").is_err());
    }

    #[test]
    fn quote_inside_unquoted_field_is_error() {
        assert!(parse_csv_line("ab\"c").is_err());
        assert!(parse_csv_line("a\"b\",c").is_err());
    }

    #[test]
    fn junk_after_closing_quote_is_error() {
        assert!(parse_csv_line("\"ab\"c").is_err());
        assert!(parse_csv_line("\"ab\" ").is_err());
        assert!(parse_csv_line("\"ab\"\"c").is_err());
    }

    #[test]
    fn errors_return_string() {
        let e = parse_csv_line("\"abc").unwrap_err();
        assert!(!e.is_empty());
    }
}
