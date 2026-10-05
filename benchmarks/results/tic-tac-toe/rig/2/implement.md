All tests pass.

## Summary

**Changed:** `src/lib.rs` (only file touched — `Cargo.toml` untouched).

**Implementation:**
- `pub fn play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>` plus a private `is_winner` helper and a `pub const EMPTY: char = '.'`.
- Validation order (each returns `Err(String)` **before** any mutation, so the board is byte-for-byte unchanged on rejection):
  1. player must be `'X'` or `'O'` (rejects `'x'`, `'y'`, `'.'`, `' '`, etc.),
  2. `row < 3` and `col < 3` (also handles `usize::MAX` without overflow),
  3. target cell must be `'.'`.
- Valid move writes the cell, then checks the mover's win across all 3 rows, all 3 columns and both diagonals → `Ok(Some(player))`; else full board (no `'.'` left) → `Ok(Some('D'))`; else `Ok(None)`. No alternation is enforced.

**Tests (14 unit tests, all green):** empty-board moves, non-alternating players, invalid-player / out-of-bounds / occupied-cell rejections each asserting the board is unchanged, all 3 row wins and all 3 column wins for both players, main and anti diagonal wins, win-beats-draw on the final cell, draw on the final move, a full 9-move draw sequence asserting `None` until the last move, a `None` case with free cells remaining, and exact `Err` message verification.

**Repairs made:** two initial test fixtures were wrong, not the library. One "draw" sequence accidentally completed the main diagonal for `'X'` (replaced with the classic `X O X / X O O / O X X` draw order), and one "priority" test wrongly assumed a pre-existing line for the *other* player counts (rewritten so the mover's own move completes a line and fills the board, proving the win is reported instead of `'D'`). Also changed a `None` fixture whose intended move actually completed column 1.

The `test` task reports PASS: 14 passed, plus 4 other tests in the workspace also passing.