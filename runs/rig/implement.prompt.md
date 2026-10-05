# Implement

Implement the plan below for this request as one self-contained Rust code block with unit tests. Return only the code block.

Request:
Build a tiny Rust tic-tac-toe game. Support legal moves, reject occupied cells, detect row, column, and diagonal wins, detect a draw, and include unit tests for those behaviors.


Plan:
- **Board + move API**: Implement a 3×3 board with `make_move(row, col, player)` returning `Ok`/`Err`; reject moves on occupied cells and out-of-range indices.
- **Win detection**: After each move, check the placed player's row, column, and both diagonals for three-in-a-row; return the winner.
- **Draw detection**: If the board is full and no winner exists, report a draw.
- **Game loop / CLI**: Simple `main` that alternates X and O, prints the board, prompts for coordinates, and handles invalid input by re-prompting.
- **Unit tests**: Cover legal moves, occupied-cell rejection, each win type (row, col, both diagonals), and a draw.

*Success criteria*: `cargo test` passes all above cases; `cargo run` plays a full game where illegal moves are rejected and terminal states (win/draw) end the loop.
