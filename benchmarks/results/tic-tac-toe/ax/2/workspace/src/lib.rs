//! A small tic-tac-toe library.
//!
//! The board is a 3x3 grid of `char` where `'.'` means empty and the
//! players are `'X'` and `'O'`.

/// Play a single move on the board.
///
/// `board` is a mutable 3x3 grid; `row`/`col` index the cell to play and
/// `player` is the piece to place (`'X'` or `'O'`).
///
/// Returns:
/// * `Err(String)` if the player is invalid, the position is out of
///   bounds, or the target cell is already occupied. The board is left
///   unchanged in every error case.
/// * `Ok(Some('X'))` / `Ok(Some('O'))` when the move completes a row,
///   column or diagonal for that player.
/// * `Ok(Some('D'))` when the board becomes full with no winner.
/// * `Ok(None)` when the game is still in progress.
pub fn play(
    board: &mut [[char; 3]; 3],
    row: usize,
    col: usize,
    player: char,
) -> Result<Option<char>, String> {
    // Validate the player before touching the board.
    if player != 'X' && player != 'O' {
        return Err(format!(
            "invalid player '{}': expected 'X' or 'O'",
            player
        ));
    }

    // Validate the position (indices are usize, so only the upper bound matters).
    if row >= 3 || col >= 3 {
        return Err(format!(
            "position ({}, {}) is out of bounds",
            row, col
        ));
    }

    // Validate the target cell.
    if board[row][col] != '.' {
        return Err(format!(
            "cell ({}, {}) is already occupied by '{}'",
            row, col, board[row][col]
        ));
    }

    // Apply the move only after all validation has passed.
    board[row][col] = player;

    // Check rows.
    for r in 0..3 {
        if board[r][0] == player && board[r][1] == player && board[r][2] == player {
            return Ok(Some(player));
        }
    }

    // Check columns.
    for c in 0..3 {
        if board[0][c] == player && board[1][c] == player && board[2][c] == player {
            return Ok(Some(player));
        }
    }

    // Check diagonals.
    if board[0][0] == player && board[1][1] == player && board[2][2] == player {
        return Ok(Some(player));
    }
    if board[0][2] == player && board[1][1] == player && board[2][0] == player {
        return Ok(Some(player));
    }

    // No winner: draw only when the board is completely full.
    for r in 0..3 {
        for c in 0..3 {
            if board[r][c] == '.' {
                return Ok(None);
            }
        }
    }

    Ok(Some('D'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> [[char; 3]; 3] {
        [['.'; 3]; 3]
    }

    #[test]
    fn valid_move_places_piece_and_returns_none() {
        let mut board = empty();
        let result = play(&mut board, 0, 0, 'X').unwrap();
        assert_eq!(result, None);
        assert_eq!(board[0][0], 'X');
    }

    #[test]
    fn invalid_player_is_rejected_without_change() {
        let mut board = empty();
        let result = play(&mut board, 1, 1, 'Z');
        assert!(result.is_err());
        assert_eq!(board, empty());
        // '.' is not a valid player either.
        assert!(play(&mut board, 1, 1, '.').is_err());
        assert_eq!(board, empty());
    }

    #[test]
    fn out_of_bounds_is_rejected_without_change() {
        let mut board = empty();
        assert!(play(&mut board, 3, 0, 'X').is_err());
        assert!(play(&mut board, 0, 3, 'X').is_err());
        assert!(play(&mut board, 5, 5, 'O').is_err());
        assert_eq!(board, empty());
    }

    #[test]
    fn occupied_cell_is_rejected_without_change() {
        let mut board = empty();
        play(&mut board, 0, 0, 'X').unwrap();
        let snapshot = board;
        let result = play(&mut board, 0, 0, 'O');
        assert!(result.is_err());
        assert_eq!(board, snapshot);
    }

    #[test]
    fn non_alternating_players_are_allowed() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X').unwrap(), None);
        assert_eq!(play(&mut board, 0, 1, 'X').unwrap(), None);
        assert_eq!(play(&mut board, 0, 2, 'X').unwrap(), Some('X'));
    }

    #[test]
    fn row_wins() {
        for r in 0..3 {
            let mut board = empty();
            assert_eq!(play(&mut board, r, 0, 'X').unwrap(), None);
            assert_eq!(play(&mut board, r, 1, 'X').unwrap(), None);
            assert_eq!(play(&mut board, r, 2, 'X').unwrap(), Some('X'));
        }
    }

    #[test]
    fn column_wins() {
        for c in 0..3 {
            let mut board = empty();
            assert_eq!(play(&mut board, 0, c, 'O').unwrap(), None);
            assert_eq!(play(&mut board, 1, c, 'O').unwrap(), None);
            assert_eq!(play(&mut board, 2, c, 'O').unwrap(), Some('O'));
        }
    }

    #[test]
    fn main_diagonal_win() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X').unwrap(), None);
        assert_eq!(play(&mut board, 1, 1, 'X').unwrap(), None);
        assert_eq!(play(&mut board, 2, 2, 'X').unwrap(), Some('X'));
    }

    #[test]
    fn anti_diagonal_win() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 2, 'O').unwrap(), None);
        assert_eq!(play(&mut board, 1, 1, 'O').unwrap(), None);
        assert_eq!(play(&mut board, 2, 0, 'O').unwrap(), Some('O'));
    }

    #[test]
    fn full_board_without_winner_is_draw() {
        // X O X
        // X O O
        // O X X
        let moves: [(usize, usize, char); 9] = [
            (0, 0, 'X'),
            (0, 1, 'O'),
            (0, 2, 'X'),
            (1, 0, 'X'),
            (1, 1, 'O'),
            (1, 2, 'O'),
            (2, 0, 'O'),
            (2, 1, 'X'),
            (2, 2, 'X'),
        ];
        let mut board = empty();
        let mut last = None;
        for (r, c, p) in moves {
            last = Some(play(&mut board, r, c, p).unwrap());
        }
        assert_eq!(last, Some(Some('D')));
    }

    #[test]
    fn ongoing_game_returns_none() {
        let mut board = empty();
        assert_eq!(play(&mut board, 0, 0, 'X').unwrap(), None);
        assert_eq!(play(&mut board, 1, 1, 'O').unwrap(), None);
        assert_eq!(play(&mut board, 2, 2, 'X').unwrap(), None);
    }

    #[test]
    fn win_is_detected_even_when_board_would_otherwise_be_full() {
        // Board is nearly full; final move completes the anti-diagonal.
        // X X O
        // O O X
        // X X O   -> but the last move makes 2,0 / 2,1 / 2,2? use a win line.
        let moves: [(usize, usize, char); 9] = [
            (0, 0, 'X'),
            (0, 1, 'X'),
            (0, 2, 'O'),
            (1, 0, 'O'),
            (1, 1, 'O'),
            (1, 2, 'X'),
            (2, 0, 'X'),
            (2, 1, 'X'),
            (2, 2, 'X'),
        ];
        let mut board = empty();
        let mut last = None;
        for (r, c, p) in moves {
            last = Some(play(&mut board, r, c, p).unwrap());
        }
        // The bottom row X X X is completed by the final move.
        assert_eq!(last, Some(Some('X')));
    }
}
