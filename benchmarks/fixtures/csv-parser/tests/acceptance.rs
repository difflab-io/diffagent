use generated_example::parse_csv_line;

fn fields(input: &str) -> Vec<String> {
    parse_csv_line(input).unwrap()
}

#[test]
fn simple_and_empty_fields() {
    assert_eq!(fields("a,b,c"), ["a", "b", "c"]);
    assert_eq!(fields(""), [""]);
    assert_eq!(fields(","), ["", ""]);
    assert_eq!(fields("a,,"), ["a", "", ""]);
}

#[test]
fn quoted_commas_and_escaped_quotes() {
    assert_eq!(fields("\"a,b\",c"), ["a,b", "c"]);
    assert_eq!(fields("\"a\"\"b\",z"), ["a\"b", "z"]);
    assert_eq!(fields("\"\",\"x\""), ["", "x"]);
}

#[test]
fn whitespace_is_preserved() {
    assert_eq!(fields(" a,\" b \""), [" a", " b "]);
    assert_eq!(fields("x, "), ["x", " "]);
}

#[test]
fn malformed_quotes_are_errors() {
    for bad in ["\"unfinished", "ab\"cd", "\"closed\"tail", "\"a\" \n"] {
        assert!(parse_csv_line(bad).is_err(), "accepted {bad:?}");
    }
}
