```rust
// src/lib.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    X,
    O,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    InProgress,
    Win(Player),
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    OutOfBounds,
    Occupied,
}

#[derive(Debug, Clone)]
pub struct Board {
    cells: [[Option<Player>; 3]; 3],
}

impl Board {
    pub fn new() -> Self {
        Board {
            cells: [[None; 3]; 3],
        }
    }

    pub fn get(&self, row: usize, col: usize) -> Option<Player> {
        self.cells[row][col]
    }

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

    pub fn state(&self) -> GameState {
        // Rows
        for r in 0..3 {
            if let Some(p) = self.cells[r][0] {
                if self.cells[r][1] == Some(p) && self.cells[r][2] == Some(p) {
                    return GameState::Win(p);
                }
            }
        }
        // Columns
        for c in 0..3 {
            if let Some(p) = self.cells[0][c] {
                if self.cells[1][c] == Some(p) && self.cells[2][c] == Some(p) {
                    return GameState::Win(p);
                }
            }
        }
        // Diagonals
        if let Some(p) = self.cells[0][0] {
            if self.cells[1][1] == Some(p) && self.cells[2][2] == Some(p) {
                return GameState::Win(p);
            }
        }
        if let Some(p) = self.cells[0][2] {
            if self.cells[1][1] == Some(p) && self.cells[2][0] == Some(p) {
                return GameState::Win(p);
            }
        }
        // Draw
        let full = self
            .cells
            .iter()
            .all(|row| row.iter().all(|c| c.is_some()));
        if full {
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
    fn legal_move_places_piece() {
        let mut b = Board::new();
        assert_eq!(b.play(0, 0, Player::X), Ok(()));
        assert_eq!(b.get(0, 0), Some(Player::X));
    }

    #[test]
    fn rejects_occupied_cell() {
        let mut b = Board::new();
        b.play(1, 1, Player::X).unwrap();
        assert_eq!(b.play(1, 1, Player::O), Err(MoveError::Occupied));
        assert_eq!(b.get(1, 1), Some(Player::X));
    }

    #[test]
    fn rejects_out_of_bounds() {
        let mut b = Board::new();
        assert_eq!(b.play(3, 0, Player::X), Err(MoveError::OutOfBounds));
        assert_eq!(b.play(0, 3, Player::X), Err(MoveError::OutOfBounds));
    }

    #[test]
    fn detects_row_wins() {
        for r in 0..3 {
            let mut b = Board::new();
            b.play(r, 0, Player::X).unwrap();
            b.play(r, 1, Player::X).unwrap();
            b.play(r, 2, Player::X).unwrap();
            assert_eq!(b.state(), GameState::Win(Player::X));
        }
    }

    #[test]
    fn detects_column_wins() {
        for c in 0..3 {
            let mut b = Board::new();
            b.play(0, c, Player::O).unwrap();
            b.play(1, c, Player::O).unwrap();
            b.play(2, c, Player::O).unwrap();
            assert_eq!(b.state(), GameState::Win(Player::O));
        }
    }

    #[test]
    fn detects_main_diagonal_win() {
        let mut b = Board::new();
        b.play(0, 0, Player::X).unwrap();
        b.play(1, 1, Player::X).unwrap();
        b.play(2, 2, Player::X).unwrap();
        assert_eq!(b.state(), GameState::Win(Player::X));
    }

    #[test]
    fn detects_anti_diagonal_win() {
        let mut b = Board::new();
        b.play(0, 2, Player::O).unwrap();
        b.play(1, 1, Player::O).unwrap();
        b.play(2, 0, Player::O).unwrap();
        assert_eq!(b.state(), GameState::Win(Player::O));
    }

    #[test]
    fn detects_draw() {
        let mut b = Board::new();
        // X O X
        // X O O
        // O X X
        let moves = [
            (0, 0, Player::X),
            (0, 1, Player::O),
            (0, 2, Player::X),
            (1, 0, Player::X),
            (1, 1, Player::O),
            (1, 2, Player::O),
            (2, 0, Player::O),
            (2, 1, Player::X),
            (2, 2, Player::X),
        ];
        for (r, c, p) in moves {
            b.play(r, c, p).unwrap();
        }
        assert_eq!(b.state(), GameState::Draw);
    }

    #[test]
    fn detects_in_progress() {
        let mut b = Board::new();
        b.play(0, 0, Player::X).unwrap();
        assert_eq!(b.state(), GameState::InProgress);
    }
}

// src/main.rs
use std::io::{self, Write};

use tic_tac_toe::{Board, GameState, MoveError, Player};

fn print_board(board: &Board) {
    for r in 0..3 {
        for c in 0..3 {
            let ch = match board.get(r, c) {
                Some(Player::X) => 'X',
                Some(Player::O) => 'O',
                None => ' ',
            };
            if c < 2 {
                print!(" {} |", ch);
            } else {
                print!(" {}", ch);
            }
        }
        println!();
        if r < 2 {
            println!("---+---+---");
        }
    }
}

fn main() {
    let mut board = Board::new();
    let mut current = Player::X;

    loop {
        print_board(&board);
        match board.state() {
            GameState::Win(p) => {
                println!("{:?} wins!", p);
                break;
            }
            GameState::Draw => {
                println!("It's a draw!");
                break;
            }
            GameState::InProgress => {}
        }

        print!("Player {:?}, enter row and column (0-2): ", current);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input.");
            continue;
        }
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 2 {
            println!("Please enter two numbers.");
            continue;
        }
        let row: usize = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid row.");
                continue;
            }
        };
        let col: usize = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid column.");
                continue;
            }
        };

        match board.play(row, col, current) {
            Ok(()) => {
                current = match current {
                    Player::X => Player::O,
                    Player::O => Player::X,
                };
            }
            Err(MoveError::OutOfBounds) => println!("Out of bounds. Try again."),
            Err(MoveError::Occupied) => println!("Cell already occupied. Try again."),
        }
    }
}
```