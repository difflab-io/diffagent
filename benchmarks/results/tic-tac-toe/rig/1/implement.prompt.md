# Implement with tools

You are in a fresh Rust library workspace. Use the available tools; do not merely return code as text. First call read_file with path `src/lib.rs`, then write_file with path `src/lib.rs` and the complete implementation including unit tests. Call run_task with task `test` to run the real tests. If tests fail, read_file, edit with write_file, and call run_task again. Continue until tests pass or you cannot fix them. Do not edit Cargo.toml. Finish with a short summary of what you actually changed and tested.

Request:
Implement a small Rust tic-tac-toe library. Public API: `pub fn play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>`. `'.'` means empty; players are `'X'` and `'O'`. Reject invalid players, out-of-bounds positions and occupied cells without changing the board. After a valid move, return `Some('X')` or `Some('O')` for a row, column or diagonal win; `Some('D')` for a full-board draw without a winner; otherwise `None`. The supplied player need not alternate. Include your own unit tests. Use the provided tools to read, write, run tests and repair failures.


Plan:
- Read the existing crate structure (`Cargo.toml`, `src/lib.rs`) and confirm the test command is `cargo test`.
- Implement `pub fn play(...)` with validation for `'X'`/`'O'`, bounds `row < 3 && col < 3`, and `'.'`-only occupancy; invalid calls return `Err` without mutating the board.
- After a valid move, check the affected row, column, and both diagonals for a win; otherwise return `Some('D')` only when the board is full, else `Ok(None)`.
- Add unit tests covering valid moves, invalid player, out-of-bounds, occupied cell/board unchanged, X/O wins by row/column/diagonal, draw, and non-alternating players.
- Run `cargo test`; success means the public signature matches, all invalid cases leave the board unchanged, all win/draw/none outcomes match the spec, and every test passes.
