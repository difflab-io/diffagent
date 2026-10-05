# Implement with tools

You are in a fresh Rust library workspace. Use the available tools; do not merely return code as text. First call read_file with path `src/lib.rs`, then write_file with path `src/lib.rs` and the complete implementation including unit tests. Call run_task with task `test` to run the real tests. If tests fail, read_file, edit with write_file, and call run_task again. Continue until tests pass or you cannot fix them. Do not edit Cargo.toml. Finish with a short summary of what you actually changed and tested.

Request:
Implement a small Rust tic-tac-toe library. Public API: `pub fn play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>`. `'.'` means empty; players are `'X'` and `'O'`. Reject invalid players, out-of-bounds positions and occupied cells without changing the board. After a valid move, return `Some('X')` or `Some('O')` for a row, column or diagonal win; `Some('D')` for a full-board draw without a winner; otherwise `None`. The supplied player need not alternate. Include your own unit tests. Use the provided tools to read, write, run tests and repair failures.


Plan:
1. Create a Rust library crate exposing `play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>`. 2. Implement validation first: reject players other than `'X'`/`'O'`, `row`/`col` >= 3, or occupied cells, returning `Err(String)` and leaving the board unchanged. 3. On valid moves, write the player, then check row/column/diagonal wins returning `Some(player)`, full board with no winner returning `Some('D')`, otherwise `None`. 4. Add unit tests covering valid moves, wins in rows/columns/diagonals, draw, invalid players, out-of-bounds, occupied cells, and non-alternating players. 5. Success criteria: `cargo test` passes; tests prove rejected moves do not mutate the board and every specified return value is produced.
