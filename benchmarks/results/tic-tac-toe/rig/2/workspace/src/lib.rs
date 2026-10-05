//! A small tic-tac-toe library.
//!
//! The board is a `3x3` grid of `char`s where `'.'` denotes an empty cell and
//! the players are `'X'` and `'O'`. The single entry point [`play`] validates a
//! move, applies it and reports the resulting game state.

/// The empty-cell marker.
pub const EMPTY: char = '.';

/// Plays `player` at `(row, col)` on `board`.
///
/// Returns `Err` (leaving the board untouched) when:
/// * `player` is neither `'X'` nor `'O'`,
/// * `row` or `col` is out of bounds (>= 3),
/// * the target cell is already occupied.
///
/// On a valid move the board is updated and the outcome is reported as:
/// * `Some('X')` / `Some('O')` when that player completed a row, column or
///   diagonal,
/// * `Some('D')` when the board is full and nobody won,
/// * `None` otherwise.
///
/// Players need not alternate; any valid player may move at any time.
pub fn play(
    board: &mut [[char; 3]; 3],
    row: usize,
    col: usize,
    player: char,
) -> Result<Option<char>, String> {
    if player != 'X' && player != 'O' {
        return Err(format!("invalid player {player:?}: expected 'X' or 'O'"));
    }

    if row >= 3 {
        return Err(format!("row {row} is out of bounds: must be 0..3"));
    }

    if col >= 3 {
        return Err(format!("column {col} is out of bounds: must be 0..3"));
    }

    if board[row][col] != EMPTY {
        return Err(format!(
            "cell ({row}, {col}) is already occupied by {:?}",
            board[row][col]
        ));
    }

    board[row][col] = player;

    if is_winner(board, player) {
        return Ok(Some(player));
    }

    if board.iter().all(|r| r.iter().all(|&c| c != EMPTY)) {
        return Ok(Some('D'));
    }

    Ok(None)
}

/// Returns `true` if `player` occupies a complete row, column or diagonal.
fn is_winner(board: &[[char; 3]; 3], player: char) -> bool {
    let row_win = (0..3).any(|r| (0..3).all(|c| board[r][c] == player));
    if row_win {
        return true;
    }

    let col_win = (0..3).any(|c| (0..3).all(|r| board[r][c] == player));
    if col_win {
        return true;
    }

    let main_diag = (0..3).all(|i| board[i][i] == player);
    if main_diag {
        return true;
    }

    (0..3).all(|i| board[i][2 - i] == player)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> [[char; 3]; 3] {
        [[EMPTY; 3]; 3]
    }

    #[test]
    fn empty_board_moves_return_none() {
        let mut b = empty();
        assert_eq!(play(&mut b, 0, 0, 'X'), Ok(None));
        assert_eq!(b[0][0], 'X');
        assert_eq!(play(&mut b, 0, 1, 'O'), Ok(None));
        assert_eq!(b[0][1], 'O');
    }

    #[test]
    fn does_not_require_alternation() {
        let mut b = empty();
        assert_eq!(play(&mut b, 0, 0, 'X'), Ok(None));
        assert_eq!(play(&mut b, 1, 1, 'X'), Ok(None));
        // Third 'X' completes the main diagonal.
        assert_eq!(play(&mut b, 2, 2, 'X'), Ok(Some('X')));
        assert_eq!(b[2][2], 'X');
    }

    #[test]
    fn rejects_invalid_player_without_touching_board() {
        let mut b = empty();
        let before = b;
        assert!(play(&mut b, 0, 0, 'x').is_err());
        assert!(play(&mut b, 0, 0, 'y').is_err());
        assert!(play(&mut b, 0, 0, EMPTY).is_err());
        assert!(play(&mut b, 0, 0, ' ').is_err());
        assert_eq!(b, before);
    }

    #[test]
    fn rejects_out_of_bounds_without_touching_board() {
        let mut b = empty();
        let before = b;
        assert!(play(&mut b, 3, 0, 'X').is_err());
        assert!(play(&mut b, 0, 3, 'X').is_err());
        assert!(play(&mut b, usize::MAX, 0, 'O').is_err());
        assert!(play(&mut b, 0, usize::MAX, 'O').is_err());
        assert_eq!(b, before);
    }

    #[test]
    fn rejects_occupied_cell_without_touching_board() {
        let mut b = empty();
        assert_eq!(play(&mut b, 1, 1, 'X'), Ok(None));
        let before = b;
        assert!(play(&mut b, 1, 1, 'O').is_err());
        assert!(play(&mut b, 1, 1, 'X').is_err());
        assert_eq!(b, before);
        assert_eq!(b[1][1], 'X');
    }

    #[test]
    fn detects_all_row_wins_for_both_players() {
        for player in ['X', 'O'] {
            for r in 0..3 {
                let mut b = empty();
                for c in 0..2 {
                    b[r][c] = player;
                }
                assert_eq!(play(&mut b, r, 2, player), Ok(Some(player)));
            }
        }
    }

    #[test]
    fn detects_all_column_wins_for_both_players() {
        for player in ['X', 'O'] {
            for c in 0..3 {
                let mut b = empty();
                for r in 0..2 {
                    b[r][c] = player;
                }
                assert_eq!(play(&mut b, 2, c, player), Ok(Some(player)));
            }
        }
    }

    #[test]
    fn detects_main_diagonal_win() {
        let mut b = empty();
        b[0][0] = 'O';
        b[1][1] = 'O';
        assert_eq!(play(&mut b, 2, 2, 'O'), Ok(Some('O')));
    }

    #[test]
    fn detects_anti_diagonal_win() {
        let mut b = empty();
        b[0][2] = 'X';
        b[1][1] = 'X';
        assert_eq!(play(&mut b, 2, 0, 'X'), Ok(Some('X')));
    }

    #[test]
    fn win_takes_priority_over_draw_on_final_cell() {
        // Only (0, 2) is free; filling it completes row 0 *and* the board,
        // so the win must be reported rather than a draw.
        let mut b = [['X', 'X', '.'], ['O', 'X', 'X'], ['O', 'O', 'X']];
        assert_eq!(play(&mut b, 0, 2, 'X'), Ok(Some('X')));
        assert!(b.iter().all(|r| r.iter().all(|&c| c != EMPTY)));
    }

    #[test]
    fn detects_draw_on_final_move() {
        // Full board with no winner once (2, 2) is filled.
        let mut b = [['X', 'O', 'X'], ['X', 'O', 'O'], ['O', 'X', '.']];
        assert_eq!(play(&mut b, 2, 2, 'X'), Ok(Some('D')));
    }

    #[test]
    fn draw_sequence_returns_none_until_board_is_full() {
        let mut b = empty();
        let moves: [(usize, usize, char); 9] = [
            (0, 0, 'X'),
            (0, 1, 'O'),
            (0, 2, 'X'),
            (1, 1, 'O'),
            (1, 0, 'X'),
            (1, 2, 'O'),
            (2, 0, 'O'),
            (2, 1, 'X'),
            (2, 2, 'X'),
        ];
        for (i, (r, c, p)) in moves.iter().enumerate() {
            let expected = if i == moves.len() - 1 {
                Ok(Some('D'))
            } else {
                Ok(None)
            };
            assert_eq!(play(&mut b, *r, *c, *p), expected, "move {i}");
        }
        assert_eq!(b, [['X', 'O', 'X'], ['X', 'O', 'O'], ['O', 'X', 'X']]);
    }

    #[test]
    fn no_outcome_while_moves_remain() {
        // Non-winning move that does not fill the board.
        let mut b = [['X', 'O', 'X'], ['X', 'O', 'O'], ['O', '.', '.']];
        assert_eq!(play(&mut b, 2, 1, 'X'), Ok(None));
        assert_eq!(b[2][1], 'X');
    }

    #[test]
    fn board_state_reflects_accepted_moves_only() {
        let mut b = empty();
        assert_eq!(play(&mut b, 0, 2, 'O'), Ok(None));
        assert_eq!(
            play(&mut b, 0, 2, 'X'),
            Err(String::from("cell (0, 2) is already occupied by 'O'"))
        );
        assert_eq!(b[0][2], 'O');
        assert_eq!(b[0][0], EMPTY);
    }
}
