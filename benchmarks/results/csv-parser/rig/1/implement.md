All tests pass.

## Summary

**What I changed:** I replaced the placeholder `src/lib.rs` with a complete implementation of `pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String>`, plus an extensive `#[cfg(test)]` test module. I did **not** touch `Cargo.toml`.

**Implementation approach:**
- Collected the input into a `Vec<char>` so multi-byte UTF-8 is handled safely while treating `,` and `"` as single significant characters.
- A per-field state machine:
  - **Quoted field** (only when `"` is the field's first char): consume the opening quote, copy contents, treat `""` as a literal `"`, and require the closing quote to be followed by end-of-input or a comma (otherwise `Err`).
  - **Unquoted field**: copy until a comma or end-of-input; any `"` encountered is an error.
- Loop structure naturally handles empty input (→ one empty field) and trailing commas (→ an extra empty field). Whitespace is copied verbatim, never trimmed.

**Behavior verified by tests (29 total, all passing):**
- Valid: empty input, single field, simple comma lists, trailing comma, lone comma, empty middle fields, quoted fields with embedded commas, quoted empty field, `""` escape, whitespace preservation (unquoted and quoted), Unicode (both plain and quoted with commas), and mixes of quoted/unquoted fields.
- Errors: unclosed quote (including the `"a""` escape-then-unterminated case), lone `"`, quote inside an unquoted field, quote not at field start, and any non-comma character after a closing quote (including trailing whitespace).

`run_task` reported **PASS** with 25 + 4 tests passing and 0 failures.

To keep the main loop correct, the code relies on all field characters (quotes, commas, and multi-byte ones) being pushed as `char`s, so UTF-8 boundaries are never split.