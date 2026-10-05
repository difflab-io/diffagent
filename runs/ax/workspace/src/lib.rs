//! A tiny tic-tac-toe game engine.
//!
//! The board is a 3x3 grid of `Option<Player>` cells. Legal moves fill an
//! empty, in-bounds cell. The engine can detect row, column, and diagonal
//! wins as well as draws.

/// The two players in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    X,
    O,
}

/// The outcome of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// The game is still in progress.
    InProgress,
    /// The given player has won.
    Win(Player),
    /// The board is full with no winner.
    Draw,
}

/// Errors that can occur when attempting a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// The requested coordinate is outside the 3x3 board.
    OutOfBounds,
    /// The requested cell is already occupied.
    Occupied,
}

/// A 3x3 tic-tac-toe board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    cells: [[Option<Player>; 3]; 3],
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    /// Create an empty board.
    pub fn new() -> Self {
        Board {
            cells: [[None; 3]; 3],
        }
    }

    /// Read the contents of a cell, if the coordinate is in bounds.
    pub fn get(&self, row: usize, col: usize) -> Option<Player> {
        self.cells.get(row).and_then(|r| r.get(col)).copied().flatten()
    }

    /// Attempt to place `player`'s mark at `(row, col)`.
    ///
    /// Returns `Err(MoveError::OutOfBounds)` for coordinates outside the board
    /// and `Err(MoveError::Occupied)` when the cell is already taken. On error
    /// the board is left unchanged.
    pub fn play(&mut self, row: usize, col: usize, player: Player) -> Result<(), MoveError> {
        if row >= 3 || col >= 3 {
            return Err(MoveError::OutOfBounds);
        }
        if self.cells[row][col].is_some() {
            return Err(MoveError::Occupied);
        }
        self.cells[row][col] = Some(player);
        Ok(())
    }

    /// Return the winner, if any.
    pub fn winner(&self) -> Option<Player> {
        const LINES: [[(usize, usize); 3]; 8] = [
            // Rows
            [(0, 0), (0, 1), (0, 2)],
            [(1, 0), (1, 1), (1, 2)],
            [(2, 0), (2, 1), (2, 2)],
            // Columns
            [(0, 0), (1, 0), (2, 0)],
            [(0, 1), (1, 1), (2, 1)],
            [(0, 2), (1, 2), (2, 2)],
            // Diagonals
            [(0, 0), (1, 1), (2, 2)],
            [(0, 2), (1, 1), (2, 0)],
        ];

        for line in LINES.iter() {
            let a = self.cells[line[0].0][line[0].1];
            let b = self.cells[line[1].0][line[1].1];
            let c = self.cells[line[2].0][line[2].1];
            if a.is_some() && a == b && b == c {
                return a;
            }
        }
        None
    }

    /// True when every cell is filled.
    pub fn is_full(&self) -> bool {
        self.cells.iter().all(|row| row.iter().all(|c| c.is_some()))
    }

    /// The current state of the game.
    pub fn state(&self) -> GameState {
        if let Some(winner) = self.winner() {
            GameState::Win(winner)
        } else if self.is_full() {
            GameState::Draw
        } else {
            GameState::InProgress
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_board_is_empty() {
        let board = Board::new();
        for row in 0..3 {
            for col in 0..3 {
                assert_eq!(board.get(row, col), None);
            }
        }
        assert_eq!(board.state(), GameState::InProgress);
    }

    #[test]
    fn legal_move_places_mark() {
        let mut board = Board::new();
        assert_eq!(board.play(0, 0, Player::X), Ok(()));
        assert_eq!(board.get(0, 0), Some(Player::X));
    }

    #[test]
    fn occupied_cell_is_rejected_and_board_unchanged() {
        let mut board = Board::new();
        board.play(1, 1, Player::X).unwrap();
        assert_eq!(board.play(1, 1, Player::O), Err(MoveError::Occupied));
        assert_eq!(board.get(1, 1), Some(Player::X));
    }

    #[test]
    fn out_of_bounds_is_rejected() {
        let mut board = Board::new();
        assert_eq!(board.play(3, 0, Player::X), Err(MoveError::OutOfBounds));
        assert_eq!(board.play(0, 3, Player::X), Err(MoveError::OutOfBounds));
        assert_eq!(board.get(0, 0), None);
    }

    fn play_all(board: &mut Board, moves: &[(usize, usize, Player)]) {
        for &(r, c, p) in moves {
            board.play(r, c, p).unwrap();
        }
    }

    #[test]
    fn detects_each_row_win() {
        for row in 0..3 {
            let mut board = Board::new();
            play_all(
                &mut board,
                &[(row, 0, Player::X), (row, 1, Player::X), (row, 2, Player::X)],
            );
            assert_eq!(board.winner(), Some(Player::X));
            assert_eq!(board.state(), GameState::Win(Player::X));
        }
    }

    #[test]
    fn detects_each_column_win() {
        for col in 0..3 {
            let mut board = Board::new();
            play_all(
                &mut board,
                &[(0, col, Player::O), (1, col, Player::O), (2, col, Player::O)],
            );
            assert_eq!(board.winner(), Some(Player::O));
            assert_eq!(board.state(), GameState::Win(Player::O));
        }
    }

    #[test]
    fn detects_both_diagonal_wins() {
        let mut main = Board::new();
        play_all(
            &mut main,
            &[(0, 0, Player::X), (1, 1, Player::X), (2, 2, Player::X)],
        );
        assert_eq!(main.winner(), Some(Player::X));

        let mut anti = Board::new();
        play_all(
            &mut anti,
            &[(0, 2, Player::O), (1, 1, Player::O), (2, 0, Player::O)],
        );
        assert_eq!(anti.winner(), Some(Player::O));
    }

    #[test]
    fn detects_draw() {
        let mut board = Board::new();
        play_all(
            &mut board,
            &[
                (0, 0, Player::X),
                (0, 1, Player::O),
                (0, 2, Player::X),
                (1, 0, Player::X),
                (1, 1, Player::O),
                (1, 2, Player::O),
                (2, 0, Player::O),
                (2, 1, Player::X),
                (2, 2, Player::X),
            ],
        );
        assert!(board.is_full());
        assert_eq!(board.winner(), None);
        assert_eq!(board.state(), GameState::Draw);
    }

    #[test]
    fn full_board_reports_in_progress_when_win_exists() {
        let mut board = Board::new();
        play_all(
            &mut board,
            &[
                (0, 0, Player::X),
                (0, 1, Player::X),
                (0, 2, Player::X),
                (1, 0, Player::O),
                (1, 1, Player::O),
                (2, 0, Player::O),
                (2, 2, Player::O),
                (1, 2, Player::X),
                (2, 1, Player::X),
            ],
        );
        assert!(board.is_full());
        assert_eq!(board.state(), GameState::Win(Player::X));
    }
}
