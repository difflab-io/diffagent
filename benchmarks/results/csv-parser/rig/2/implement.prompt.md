# Implement with tools

You are in a fresh Rust library workspace. Use the available tools; do not merely return code as text. First call read_file with path `src/lib.rs`, then write_file with path `src/lib.rs` and the complete implementation including unit tests. Call run_task with task `test` to run the real tests. If tests fail, read_file, edit with write_file, and call run_task again. Continue until tests pass or you cannot fix them. Do not edit Cargo.toml. Finish with a short summary of what you actually changed and tested.

Request:
Implement a small Rust library with the public API `pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String>`. Parse one CSV record: commas separate fields, a field may be quoted only from its first character, commas inside quotes are literal, and `""` inside a quoted field means one literal `"`. Reject an unclosed quote, a quote inside an unquoted field, and any characters after a closing quote other than a comma. Whitespace is data (do not trim it). An empty input means one empty field; a trailing comma adds an empty field. Do not implement multi-line records. Include your own unit tests. Use the provided tools to read, write, run tests and repair failures.


Plan:
- Create a Rust library crate exposing `pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String>` in `src/lib.rs`.
- Implement a character-by-character state machine for unquoted, quoted, and post-closing-quote field states, preserving whitespace and handling empty input/trailing commas correctly.
- Return `Err(String)` for unclosed quotes, quotes in unquoted fields, and any non-comma character after a closing quote.
- Add unit tests covering normal fields, empty input, trailing commas, quoted commas, doubled quotes, whitespace preservation, and every specified error case.
- Run `cargo test`; success means the crate compiles, all tests pass, and observed outputs match the required API and parsing rules.
