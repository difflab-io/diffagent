# Plan

Make a short implementation plan for the user's request. Include observable success criteria. Keep it to five bullets.

Request:
Implement a small Rust tic-tac-toe library. Public API: `pub fn play(board: &mut [[char; 3]; 3], row: usize, col: usize, player: char) -> Result<Option<char>, String>`. `'.'` means empty; players are `'X'` and `'O'`. Reject invalid players, out-of-bounds positions and occupied cells without changing the board. After a valid move, return `Some('X')` or `Some('O')` for a row, column or diagonal win; `Some('D')` for a full-board draw without a winner; otherwise `None`. The supplied player need not alternate. Include your own unit tests. Use the provided tools to read, write, run tests and repair failures.

