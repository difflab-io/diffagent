//! A tiny single-line CSV record parser.
//!
//! Rules:
//! - Commas separate fields.
//! - A field may be quoted only from its first character.
//! - Commas inside quotes are literal.
//! - `""` inside a quoted field means one literal `"`.
//! - Reject: an unclosed quote, a quote inside an unquoted field, and any
//!   character after a closing quote other than a comma.
//! - Whitespace is data; never trimmed.
//! - Empty input is one empty field; a trailing comma adds an empty field.
//! - Multi-line records are not implemented.

/// Parse a single CSV record (one line) into its fields.
///
/// Returns `Ok(fields)` on success, or `Err(message)` describing the first
/// syntax error encountered.
pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String> {
    // Work in `char`s so multi-byte UTF-8 content is handled safely; the
    // significant ASCII bytes (`,` and `"`) are always single chars.
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();

    let mut fields: Vec<String> = Vec::new();
    let mut i = 0usize;

    loop {
        let mut field = String::new();

        if i < n && chars[i] == '"' {
            // Quoted field: consume the opening quote, then read until the
            // matching closing quote, treating `""` as a literal `"`.
            i += 1;
            loop {
                if i >= n {
                    return Err("unclosed quote".to_string());
                }
                if chars[i] == '"' {
                    if i + 1 < n && chars[i + 1] == '"' {
                        // Escaped quote -> literal `"`, stay inside the field.
                        field.push('"');
                        i += 2;
                    } else {
                        // Closing quote.
                        i += 1;
                        break;
                    }
                } else {
                    field.push(chars[i]);
                    i += 1;
                }
            }

            // After the closing quote, only a comma or end-of-input is allowed.
            if i < n && chars[i] != ',' {
                return Err(format!(
                    "unexpected character '{}' after closing quote",
                    chars[i]
                ));
            }
        } else {
            // Unquoted field: read until a comma or end-of-input.
            while i < n && chars[i] != ',' {
                if chars[i] == '"' {
                    return Err("quote inside unquoted field".to_string());
                }
                field.push(chars[i]);
                i += 1;
            }
        }

        fields.push(field);

        if i >= n {
            // No more characters: if we just consumed a comma (trailing comma)
            // the loop continues once more to add the empty trailing field.
            break;
        }

        // `chars[i]` is a comma separating fields; skip it and parse the next.
        i += 1;
    }

    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::parse_csv_line;

    fn ok(input: &str) -> Vec<String> {
        parse_csv_line(input).expect("expected Ok")
    }

    fn err(input: &str) -> String {
        parse_csv_line(input).expect_err("expected Err")
    }

    // ---- Valid cases ----

    #[test]
    fn empty_input_is_one_empty_field() {
        assert_eq!(ok(""), vec!["".to_string()]);
    }

    #[test]
    fn single_unquoted_field() {
        assert_eq!(ok("hello"), vec!["hello".to_string()]);
    }

    #[test]
    fn simple_comma_separated() {
        assert_eq!(
            ok("a,b,c"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn trailing_comma_adds_empty_field() {
        assert_eq!(ok("a,"), vec!["a".to_string(), "".to_string()]);
    }

    #[test]
    fn only_comma_is_two_empty_fields() {
        assert_eq!(ok(","), vec!["".to_string(), "".to_string()]);
    }

    #[test]
    fn empty_fields_in_the_middle() {
        assert_eq!(
            ok("a,b,,c"),
            vec![
                "a".to_string(),
                "b".to_string(),
                "".to_string(),
                "c".to_string()
            ]
        );
    }

    #[test]
    fn quoted_field_with_comma_inside() {
        assert_eq!(
            ok("\"a,b\",c"),
            vec!["a,b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn quoted_empty_field() {
        assert_eq!(ok("\"\""), vec!["".to_string()]);
    }

    #[test]
    fn escaped_quote_inside_quoted_field() {
        assert_eq!(ok("\"a\"\"b\""), vec!["a\"b".to_string()]);
    }

    #[test]
    fn escaped_quote_with_surrounding_commas() {
        assert_eq!(
            ok("\"he said \"\"hi\"\"\",x"),
            vec!["he said \"hi\"".to_string(), "x".to_string()]
        );
    }

    #[test]
    fn whitespace_is_preserved() {
        assert_eq!(
            ok("  a , b ,\t c "),
            vec![
                "  a ".to_string(),
                " b ".to_string(),
                "\t c ".to_string()
            ]
        );
    }

    #[test]
    fn whitespace_preserved_inside_quotes() {
        assert_eq!(ok("\"  x  \""), vec!["  x  ".to_string()]);
    }

    #[test]
    fn quoted_field_containing_only_spaces() {
        assert_eq!(ok("\" \""), vec![" ".to_string()]);
    }

    #[test]
    fn unicode_is_preserved() {
        assert_eq!(
            ok("héllo,wörld"),
            vec!["héllo".to_string(), "wörld".to_string()]
        );
    }

    #[test]
    fn unicode_inside_quotes_with_comma() {
        assert_eq!(
            ok("\"日,本\",x"),
            vec!["日,本".to_string(), "x".to_string()]
        );
    }

    #[test]
    fn quoted_field_followed_by_end() {
        assert_eq!(ok("\"abc\""), vec!["abc".to_string()]);
    }

    #[test]
    fn multiple_quoted_and_unquoted() {
        assert_eq!(
            ok("a,\"b,c\",d,\"e\"\"f\""),
            vec![
                "a".to_string(),
                "b,c".to_string(),
                "d".to_string(),
                "e\"f".to_string()
            ]
        );
    }

    // ---- Error cases ----

    #[test]
    fn unclosed_quote_is_error() {
        let _ = err("\"abc");
    }

    #[test]
    fn lone_quote_is_error() {
        let _ = err("\"");
    }

    #[test]
    fn unclosed_quote_after_escape_is_error() {
        let _ = err("\"a\"\"");
    }

    #[test]
    fn quote_inside_unquoted_field_is_error() {
        let _ = err("ab\"cd\"");
    }

    #[test]
    fn quote_at_start_of_second_unquoted_field_is_error() {
        let _ = err("a,b\"c");
    }

    #[test]
    fn characters_after_closing_quote_are_error() {
        let _ = err("\"a\"x");
        let _ = err("\"a\" ");
        let _ = err("\"a\"b,c");
    }

    #[test]
    fn quoted_field_not_starting_at_first_char_is_error() {
        // Leading space means the field is unquoted, so the quote is illegal.
        let _ = err(" \"a\"");
    }

    #[test]
    fn error_returns_err_not_ok() {
        assert!(parse_csv_line("\"oops").is_err());
        assert!(parse_csv_line("a\"b").is_err());
        assert!(parse_csv_line("\"a\"z").is_err());
    }
}
