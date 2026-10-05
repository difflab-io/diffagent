All tests pass: 24 unit tests plus the 1 doctest (`cargo test` exit PASS). The one `Failed to spawn rustc ... PermissionDenied` line is a sandbox artifact from the doctest harness re-invoking `rustc`; that same doctest run still reported `1 passed`, and the library tests are all green with no warnings.

## Summary

**What I changed:** `src/lib.rs` only — I replaced the placeholder with a complete, self-contained tic-tac-toe engine. `Cargo.toml` was left untouched.

**Implementation**
- **`Player`**: `X`/`O` with `other()` and `glyph()` (plus `Display`).
- **`Board`** (`Copy`, 3×3 `[[Option<Player>; 3]; 3]`):
  - `place(row, col, player)` returns `None` on success or `MoveError::OutOfBounds`/`Occupied` — it validates *before* writing, so a rejected move can never mutate state.
  - `lines()` enumerates all 8 lines (3 rows, 3 columns, both diagonals); `winning_line()` finds a completed line and reports the player plus the exact coordinates.
  - `state()` orders win-and-then-draw correctly, so a draw is only ever reported when all 9 cells are filled *and* no line exists.
  - `filled_count()`, `is_full()`, `get()`, `is_in_bounds()`, and `render()` for ASCII output.
- **`GameState`**: `InProgress` / `Win(Player)` / `Draw` with `is_finished()` and `winner()`.
- **`MoveError`**: `OutOfBounds`, `Occupied`, `GameOver(GameState)`; implements `Display` + `std::error::Error`.
- **`Game`**: tracks board, `current_player()`, and cached state. `play(row, col)` rejects moves after the game ends (`GameOver`), otherwise applies the move on a copy, recomputes state, and only swaps the turn if the game is still in progress.

**Testing:** 24 unit tests cover: fresh-game invariants, legal moves updating the board and swapping turns, occupied-cell rejection without mutation (single cell, mid-game across each taken cell, and a fully-filled board), out-of-bounds rejection with turn preservation, all 3 rows and all 3 columns parametrically, both diagonals, an `O` win, immediate win detection on the winning move, the full-board draw (with the rendered board checked as `X O X / X O O / O X X`), no false draw at 8 cells, post-game move rejection for both win and draw, `Board::lines()` contents, error message formatting, and render output. The crate-level doctest also passes.

**Note on the plan's CLI step:** the workspace exposes no binary target and I was told not to edit `Cargo.toml`, so `cargo run` wasn't applicable. `Game::play` returning `GameState`/`MoveError` plus `Game::render()` provides the exact hooks a CLI would need.