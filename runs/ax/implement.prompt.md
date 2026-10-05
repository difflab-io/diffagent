# Implement

Implement the plan below for this request as one self-contained Rust code block with unit tests. Return only the code block.

Request:
Build a tiny Rust tic-tac-toe game. Support legal moves, reject occupied cells, detect row, column, and diagonal wins, detect a draw, and include unit tests for those behaviors.


Plan:
# Implementation Plan: Rust Tic-Tac-Toe

- **Scaffold project**: Create a Cargo binary crate with `src/main.rs` (game loop + CLI I/O) and `src/lib.rs` (game logic) so tests can target the library.
- **Model the board**: Define a `Board` struct with a 3×3 `[Option<Player>; 9]` (or `[[Option<Player>; 3]; 3]`), plus `Player` enum (`X`, `O`) and a `GameState` enum (`InProgress`, `Win(Player)`, `Draw`).
- **Implement moves & win detection**: Add `play(row, col)` that returns `Err` on out-of-bounds or occupied cells, and a `state()` method checking all 3 rows, 3 columns, and 2 diagonals for wins, then draw when the board is full.
- **Wire up the CLI**: In `main.rs`, alternate turns, print the board, prompt for input, surface illegal-move errors, and exit on win/draw.
- **Add unit tests** in `lib.rs` (`#[cfg(test)]`): cover legal move placement, rejection of occupied cells, each of the 8 winning lines, a draw, and an in-progress state.

**Success criteria**: `cargo test` passes all cases above; `cargo run` plays a full game interactively, refusing occupied cells and announcing the correct winner or draw.
