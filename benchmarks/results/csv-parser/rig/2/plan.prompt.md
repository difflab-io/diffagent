# Plan

Make a short implementation plan for the user's request. Include observable success criteria. Keep it to five bullets.

Request:
Implement a small Rust library with the public API `pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String>`. Parse one CSV record: commas separate fields, a field may be quoted only from its first character, commas inside quotes are literal, and `""` inside a quoted field means one literal `"`. Reject an unclosed quote, a quote inside an unquoted field, and any characters after a closing quote other than a comma. Whitespace is data (do not trim it). An empty input means one empty field; a trailing comma adds an empty field. Do not implement multi-line records. Include your own unit tests. Use the provided tools to read, write, run tests and repair failures.

