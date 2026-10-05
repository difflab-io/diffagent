//! A tiny tic-tac-toe library.
//!
//! The board is a 3x3 array of `char`s where `'.'` means empty and the two
//! players are `'X'` and `'O'`. The single entry point is [`play`].

/// Play a single move on `board`.
///
/// The move is only applied when it is valid:
///
/// * `player` must be `'X'` or `'O'`;
/// * `row`/`col` must each be less than 3;
/// * the target cell must be empty (`'.'`).
///
/// Invalid moves return `Err` with a short message and leave `board` untouched.
///
/// After a valid move the affected row, column and (where relevant) both
/// diagonals are inspected. The return value is:
///
/// * `Ok(Some('X'))` / `Ok(Some('O'))` when the move wins the game for that
///   player;
/// * `Ok(Some('D'))` when the board is now completely full with no winner;
/// * `Ok(None)` otherwise.
///
/// The player does not need to alternate.
pub fn play(
    board: &mut [[char; 3]; 3],
    row: usize,
    col: usize,
    player: char,
) -> Result<Option<char>, String> {
    if player != 'X' && player != 'O' {
        return Err(format!("invalid player: '{player}'"));
    }

    if row >= 3 || col >= 3 {
        return Err(format!("position ({row}, {col}) is out of bounds"));
    }

    if board[row][col] != '.' {
        return Err(format!("cell ({row}, {col}) is already occupied"));
    }

    board[row][col] = player;

    if is_win(board, row, col) {
        return Ok(Some(player));
    }

    if is_full(board) {
        return Ok(Some('D'));
    }

    Ok(None)
}

/// Returns `true` when the cell at `(row, col)` completes a line.
fn is_win(board: &[[char; 3]; 3], row: usize, col: usize) -> bool {
    let player = board[row][col];

    // Row.
    if board[row].iter().all(|&c| c == player) {
        return true;
    }

    // Column.
    if (0..3).all(|r| board[r][col] == player) {
        return true;
    }

    // Main diagonal (top-left to bottom-right).
    if row == col && (0..3).all(|i| board[i][i] == player) {
        return true;
    }

    // Anti diagonal (top-right to bottom-left).
    if row + col == 2 && (0..3).all(|i| board[i][2 - i] == player) {
        return true;
    }

    false
}

/// Returns `true` when every cell is filled.
fn is_full(board: &[[char; 3]; 3]) -> bool {
    board.iter().all(|line| line.iter().all(|&c| c != '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> [[char; 3]; 3] {
        [['.'; 3]; 3]
    }

    #[test]
    fn first_move_returns_none() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(board[0][0], 'X');
    }

    #[test]
    fn players_need_not_alternate() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 0, 1, 'X'), Ok(None));
        assert_eq!(board[0][0], 'X');
        assert_eq!(board[0][1], 'X');
    }

    #[test]
    fn rejects_invalid_player_without_mutating() {
        let mut board = empty();
        assert!(play(&mut board, 0, 0, 'Y').is_err());
        assert!(play(&mut board, 0, 0, '.').is_err());
        assert!(play(&mut board, 0, 0, 'x').is_err());
        assert_eq!(board, empty());
    }

    #[test]
    fn rejects_out_of_bounds_without_mutating() {
        let mut board = empty();
        assert!(play(&mut board, 3, 0, 'X').is_err());
        assert!(play(&mut board, 0, 3, 'X').is_err());
        assert!(play(&mut board, 3, 3, 'X').is_err());
        assert!(play(&mut board, usize::MAX, 0, 'O').is_err());
        assert_eq!(board, empty());
    }

    #[test]
    fn rejects_occupied_cell_without_mutating() {
        let mut board = empty();
        assert_eq!(play(&mut board, 1, 1, 'X'), Ok(None));
        let snapshot = board;
        assert!(play(&mut board, 1, 1, 'O').is_err());
        assert!(play(&mut board, 1, 1, 'X').is_err());
        assert_eq!(board, snapshot);
    }

    #[test]
    fn x_wins_by_row() {
        let mut board = empty();
        // X X X on the top row, with a couple of O moves interspersed.
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 1, 0, 'O'), Ok(None));
        assert_eq!(play(&mut board, 0, 1, 'X'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 0, 2, 'X'), Ok(Some('X')));
    }

    #[test]
    fn o_wins_by_column() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 2, 1, 'O'), Ok(Some('O')));
    }

    #[test]
    fn x_wins_by_main_diagonal() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'X'), Ok(None));
        assert_eq!(play(&mut board, 2, 2, 'X'), Ok(Some('X')));
    }

    #[test]
    fn o_wins_by_anti_diagonal() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 2, 'O'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 2, 0, 'O'), Ok(Some('O')));
    }

    #[test]
    fn win_detected_on_last_cell() {
        // A board that fills up with the winning move happening last.
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut board, 0, 1, 'O'), Ok(None));
        assert_eq!(play(&mut board, 0, 2, 'X'), Ok(None));
        assert_eq!(play(&mut board, 1, 0, 'O'), Ok(None));
        assert_eq!(play(&mut board, 1, 1, 'X'), Ok(None));
        assert_eq!(play(&mut board, 2, 0, 'O'), Ok(None));
        assert_eq!(play(&mut board, 2, 1, 'O'), Ok(None));
        // Filling (2,2) completes the X main diagonal.
        assert_eq!(play(&mut board, 2, 2, 'X'), Ok(Some('X')));
    }

    #[test]
    fn detects_draw() {
        let mut board = empty();
        // X O X
        // X O O
        // O X X  -> full board, no winner.
        let moves = [
            (0, 0, 'X'),
            (0, 1, 'O'),
            (0, 2, 'X'),
            (1, 0, 'X'),
            (1, 1, 'O'),
            (1, 2, 'O'),
            (2, 0, 'O'),
            (2, 1, 'X'),
        ];
        for (r, c, p) in moves {
            assert_eq!(play(&mut board, r, c, p), Ok(None));
        }
        assert_eq!(play(&mut board, 2, 2, 'X'), Ok(Some('D')));
    }
}
