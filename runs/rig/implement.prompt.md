# Implement with tools

You are in a fresh Rust library workspace. Use the available tools; do not merely return code as text. First call read_file with path `src/lib.rs`, then write_file with path `src/lib.rs` and the complete implementation including unit tests. Call run_task with task `test` to run the real tests. If tests fail, read_file, edit with write_file, and call run_task again. Continue until tests pass or you cannot fix them. Do not edit Cargo.toml. Finish with a short summary of what you actually changed and tested.

Request:
Build a tiny Rust tic-tac-toe game. Support legal moves, reject occupied cells, detect row, column, and diagonal wins, detect a draw, and include unit tests for those behaviors.


Plan:
- Scaffold a Cargo project with a 3×3 `Board`, `Player`, and `GameState`; success: `cargo check` passes and `cargo run` starts a playable CLI.
- Implement `play(row, col)` to accept empty cells and reject out-of-bounds or occupied cells; success: tests verify valid moves update the board and invalid moves return errors without mutation.
- After each move, detect wins across all rows, columns, and both diagonals; success: tests cover each win type and confirm the correct winner.
- Detect a draw only when all 9 cells are filled and no winning line exists; success: tests cover a full-board draw and no false draw before the board is full.
- Add unit tests for legal moves, occupied rejection, row/column/diagonal wins, and draws; success: `cargo test` passes all tests and the CLI reports win/draw correctly.
