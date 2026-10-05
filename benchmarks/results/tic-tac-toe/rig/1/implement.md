All tests pass. Here's a summary of what I did:

## Changes

**`Cargo.toml`** — read only, left unchanged (confirms the test command is `cargo test`).

**`src/lib.rs`** — replaced the placeholder with the full implementation plus unit tests.

### Implementation
- **`play(board, row, col, player) -> Result<Option<char>, String>`** validates in order:
  1. `player` must be `'X'` or `'O'` → else `Err`.
  2. `row < 3 && col < 3` → else `Err` (handles `usize::MAX` safely).
  3. target cell must be `'.'` → else `Err`.
  
  All three invalid paths return before any mutation, so the board is untouched.
- After a valid move it writes to the board, then checks the affected **row**, **column**, and relevant **diagonals** via a helper `is_win` (only the main diagonal when `row == col`, only the anti-diagonal when `row + col == 2`). A win returns `Ok(Some(player))`.
- Otherwise `is_full` decides: full board → `Ok(Some('D'))`, else `Ok(None)`.
- The supplied player is never required to alternate.

### Tests (11 unit tests)
Covering: valid first move, non-alternating players, invalid player, out-of-bounds, occupied cell — each asserting the board is unchanged — plus X/O wins by row, column, main diagonal and anti-diagonal, a win on the final cell, and a full-board draw.

### Test result
`cargo test` passes: **11 passed; 0 failed** (plus the crate's existing test targets, all green).