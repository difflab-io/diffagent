Implemented the tic-tac-toe library in `src/lib.rs` (Cargo.toml untouched).

What changed:
- Added `pub fn play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>`.
- Validation runs before any board mutation: rejects players other than `'X'`/`'O'`, out-of-bounds `row`/`col` (`>= 3`), and occupied cells, returning `Err(String)` while leaving the board unchanged.
- On a legal move it writes the player, then:
  - returns `Ok(Some(player))` for a completed row, column, or either diagonal (helper `wins`);
  - returns `Ok(Some('D'))` when the board is full with no winner;
  - otherwise `Ok(None)`.
- The player is not required to alternate.
- Added a doc example and unit tests covering: valid move persistence, row/column/main-diagonal/anti-diagonal wins, full-board draw, invalid players, out-of-bounds, occupied cells, non-alternating players, and that validation precedes mutation (board untouched on every rejected move).

Verification: `run_task` with task `test` returned PASS — 11 unit tests, 4 doctests, and 1 further test all passed with 0 failures and no compiler warnings. (The single "Failed to spawn rustc / PermissionDenied" line is a sandbox restriction on an auxiliary test binary and did not affect the overall PASS result.)