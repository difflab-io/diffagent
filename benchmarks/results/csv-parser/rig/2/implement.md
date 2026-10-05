All tests pass. 

## Summary

**What I changed**

I replaced the placeholder `src/lib.rs` with a complete CSV record parser and its own unit tests. `Cargo.toml` was left untouched.

**Implementation** (`pub fn parse_csv_line(input: &str) -> Result<Vec<String>, String>`)

A character-by-character state machine over a `chars().peekable()` iterator with four states:

- `Start` — quote here opens a quoted field; comma emits an empty field; anything else begins an unquoted field.
- `Unquoted` — comma ends the field, a `"` is rejected, other chars (including whitespace) are data.
- `Quoted` — a `"` followed by another `"` is an escaped literal `"` (consumed as a pair); otherwise it's the closing quote. Everything else, including commas, is literal.
- `AfterQuote` — only a comma is allowed to end the field; any other char is rejected.

Rules enforced exactly as specified: unclosed quote → `Err`; quote inside an unquoted field → `Err`; any non-comma after a closing quote → `Err`; no trimming (whitespace is data); empty input yields one empty field; trailing commas add empty fields; no multi-line handling.

**Tests I added and ran** (13 unit tests, all passing)

- Simple fields, quoted fields containing commas, quoted empty fields.
- Empty input, `,`, `a,,b`, `,,`, and trailing commas yielding empty fields.
- Doubled quotes: `"a""b"` → `a"b`, `""""` → `"`, embedded escaping.
- Whitespace preservation (`" a , b "`, tabs/newlines, quoted spaces).
- All three error cases, plus odd-quote-count inputs left unclosed.
- Error messages are non-empty.

`cargo test` reports **PASS** (13 library tests, all green).