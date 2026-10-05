# Implement with tools

You are in a fresh Rust library workspace. Use the available tools; do not merely return code as text. First call read_file with path `src/lib.rs`, then write_file with path `src/lib.rs` and the complete implementation including unit tests. Call run_task with task `test` to run the real tests. If tests fail, read_file, edit with write_file, and call run_task again. Continue until tests pass or you cannot fix them. Do not edit Cargo.toml. Finish with a short summary of what you actually changed and tested.

Request:
Implement a small Rust tic-tac-toe library. Public API: `pub fn play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>`. `'.'` means empty; players are `'X'` and `'O'`. Reject invalid players, out-of-bounds positions and occupied cells without changing the board. After a valid move, return `Some('X')` or `Some('O')` for a row, column or diagonal win; `Some('D')` for a full-board draw without a winner; otherwise `None`. The supplied player need not alternate. Include your own unit tests. Use the provided tools to read, write, run tests and repair failures.


Plan:
- Read/inspect the workspace, then create or update a Rust library crate with `Cargo.toml` and `src/lib.rs`; success: the crate exposes the required public `play` function.
- Implement validation for `player` (`'X'` or `'O'`), `row`/`col` in `0..3`, and empty `'.'` cells; success: invalid moves return `Err(String)` and leave the board unchanged.
- Apply valid moves and detect wins across rows, columns, and both diagonals; success: returns `Some('X')`/`Some('O')` on win, `Some('D')` on full-board draw, and `None` otherwise.
- Add unit tests covering valid moves, invalid player, out-of-bounds, occupied cells, all win lines, draws, and non-alternating players; success: tests assert both return values and resulting board state.
- Run `cargo test`, repair any failures, and rerun until green; success: all tests pass and the required API behaves exactly as specified.
