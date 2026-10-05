All tests passed on the first run: 12 unit tests passed (plus 4 doc/integration tests), with 0 failures.

Summary of what I changed and tested:

**Changed:** `src/lib.rs` — replaced the placeholder with a complete implementation of `pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String>` using a char-by-char state machine with four states:
- `Start` — quote begins a quoted field, comma ends an empty field, any other char starts an unquoted field.
- `Unquoted` — comma ends the field; a `"` here is an error.
- `Quoted` — commas are literal; `""` collapses to one `"`; a lone `"` moves to `AfterQuote`.
- `AfterQuote` — only a comma is allowed; anything else is an error.

At end-of-input, an open `Quoted` state yields `Err("unclosed quote")`, and the accumulated (possibly empty) field is pushed, which correctly produces `[""]` for empty input and appends an empty field for a trailing comma. No trimming is performed, so whitespace survives verbatim. `Cargo.toml` was left untouched.

**Tested** (via `run_task test`): empty input, simple/comma-separated fields, trailing and doubled commas, quoted commas, escaped quotes (`"a""b"`, `""""`, `""`), quote-only-at-field-start behavior, whitespace preservation, and the three error cases (unclosed quote, quote inside an unquoted field, junk after a closing quote). Result: **PASS**.