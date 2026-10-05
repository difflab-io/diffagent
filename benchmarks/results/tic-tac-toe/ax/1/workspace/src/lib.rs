//! A small tic-tac-toe library.
//!
//! The board is a 3x3 grid of `char`. `'.'` denotes an empty cell, and the
//! players are `'X'` and `'O'`.
//!
//! ## Example
//! ```
//! use generated_example::play;
//!
//! let mut board = [['.', '.', '.'], ['.', '.', '.'], ['.', '.', '.']];
//! assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
//! assert_eq!(board[0][0], 'X');
//! ```

/// Play a move on the given `board`.
///
/// Places `player` at (`row`, `col`) if the move is legal, then reports the
/// outcome of the move:
///
/// * `Ok(Some(player))` if the move wins a row, column or diagonal.
/// * `Ok(Some('D'))` if the move fills the board with no winner (draw).
/// * `Ok(None)` if the game continues.
///
/// Returns `Err(String)` and leaves the board unchanged if:
///
/// * `player` is not `'X'` or `'O'`,
/// * `row` or `col` is out of bounds (`>= 3`),
/// * the target cell is already occupied.
///
/// The supplied `player` need not alternate with the existing board state.
pub fn play(
    board: &mut [[char; 3]; 3],
    row: usize,
    col: usize,
    player: char,
) -> Result<Option<char>, String> {
    // Validate the player without touching the board.
    if player != 'X' && player != 'O' {
        return Err(format!(
            "invalid player: {player:?}; expected 'X' or 'O'"
        ));
    }

    // Validate bounds without touching the board.
    if row >= 3 || col >= 3 {
        return Err(format!("position out of bounds: ({row}, {col})"));
    }

    // Validate that the cell is empty without touching the board.
    if board[row][col] != '.' {
        return Err(format!(
            "cell ({row}, {col}) is already occupied by {:?}",
            board[row][col]
        ));
    }

    // The move is legal: apply it.
    board[row][col] = player;

    // Check for a win on the row, column and the two diagonals.
    if wins(board, row, col, player) {
        return Ok(Some(player));
    }

    // A draw is a completely full board with no winner.
    if board.iter().all(|r| r.iter().all(|&c| c != '.')) {
        return Ok(Some('D'));
    }

    Ok(None)
}

/// Returns `true` if the stone just placed at (`row`, `col`) completes a line.
fn wins(board: &[[char; 3]; 3], row: usize, col: usize, player: char) -> bool {
    // Row.
    if board[row].iter().all(|&c| c == player) {
        return true;
    }

    // Column.
    if (0..3).all(|r| board[r][col] == player) {
        return true;
    }

    // Main diagonal.
    if row == col && (0..3).all(|i| board[i][i] == player) {
        return true;
    }

    // Anti-diagonal.
    if row + col == 2 && (0..3).all(|i| board[i][2 - i] == player) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: [[char; 3]; 3] = [['.', '.', '.'], ['.', '.', '.'], ['.', '.', '.']];

    #[test]
    fn valid_move_places_player_and_continues() {
        let mut board = EMPTY;
        assert_eq!(play(&mut board, 1, 1, 'X'), Ok(None));
        assert_eq!(board[1][1], 'X');
    }

    #[test]
    fn row_win_returns_winner() {
        let mut board = EMPTY;
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 0, 1, 'X'), Ok(None));
        assert_eq!(play(&mut board, 0, 2, 'X'), Ok(Some('X')));
    }

    #[test]
    fn column_win_returns_winner() {
        let mut board = EMPTY;
        assert_eq!(play(&mut board, 0, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 2, 1, 'O'), Ok(Some('O')));
    }

    #[test]
    fn main_diagonal_win_returns_winner() {
        let mut board = EMPTY;
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'X'), Ok(None));
        assert_eq!(play(&mut board, 2, 2, 'X'), Ok(Some('X')));
    }

    #[test]
    fn anti_diagonal_win_returns_winner() {
        let mut board = EMPTY;
        assert_eq!(play(&mut board, 0, 2, 'O'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 2, 0, 'O'), Ok(Some('O')));
    }

    #[test]
    fn full_board_without_winner_is_draw() {
        // A classic drawn position.
        let mut board = [['X', 'O', 'X'], ['X', 'O', 'O'], ['O', 'X', '.']];
        assert_eq!(play(&mut board, 2, 2, 'X'), Ok(Some('D')));
        assert_eq!(board[2][2], 'X');
    }

    #[test]
    fn invalid_player_is_rejected_without_mutating_board() {
        let mut board = EMPTY;
        let before = board;
        assert!(play(&mut board, 0, 0, 'Z').is_err());
        assert!(play(&mut board, 0, 0, '.').is_err());
        assert_eq!(board, before);
    }

    #[test]
    fn out_of_bounds_is_rejected_without_mutating_board() {
        let mut board = EMPTY;
        let before = board;
        assert!(play(&mut board, 3, 0, 'X').is_err());
        assert!(play(&mut board, 0, 3, 'X').is_err());
        assert!(play(&mut board, 99, 99, 'O').is_err());
        assert_eq!(board, before);
    }

    #[test]
    fn occupied_cell_is_rejected_without_mutating_board() {
        let mut board = EMPTY;
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        let before = board;
        assert!(play(&mut board, 0, 0, 'O').is_err());
        assert_eq!(board, before);
    }

    #[test]
    fn players_need_not_alternate() {
        let mut board = EMPTY;
        // Two X moves in a row.
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 0, 1, 'X'), Ok(None));
        assert_eq!(play(&mut board, 0, 2, 'X'), Ok(Some('X')));
    }

    #[test]
    fn validation_precedes_board_mutation() {
        // Even when a player is invalid *and* the position is bad, no writes happen.
        let mut board = EMPTY;
        let before = board;
        assert!(play(&mut board, 5, 5, 'Q').is_err());
        assert_eq!(board, before);
    }
}
