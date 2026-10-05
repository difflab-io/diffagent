# Implement with tools

You are in a fresh Rust library workspace. Use the available tools; do not merely return code as text. First call read_file with path `src/lib.rs`, then write_file with path `src/lib.rs` and the complete implementation including unit tests. Call run_task with task `test` to run the real tests. If tests fail, read_file, edit with write_file, and call run_task again. Continue until tests pass or you cannot fix them. Do not edit Cargo.toml. Finish with a short summary of what you actually changed and tested.

Request:
Build a tiny Rust tic-tac-toe game. Support legal moves, reject occupied cells, detect row, column, and diagonal wins, detect a draw, and include unit tests for those behaviors.


Plan:
1) Implement a minimal Rust crate with a `Board` (3×3 `Option<Player>`) and `play(row, col, player)` that accepts only legal coordinates and returns `Err` for occupied/out-of-bounds moves; success = invalid moves leave the board unchanged. 2) Add `winner()` scanning all rows, columns, and both diagonals; success = every winning line is detected for both X and O. 3) Add draw/game-status handling for a full board with no winner; success = draw is recognized only when all 9 cells are filled and no winning line exists. 4) Add `#[cfg(test)]` unit tests covering legal moves, occupied-cell rejection, row wins, column wins, diagonal wins, and draws; success = `cargo test` passes all tests. 5) Keep the public API minimal and deterministic; success = `cargo build` and `cargo test` complete without errors or warnings.
