# Evaluate

Review the implementation against the user's request and the plan. Do not run the code. Check the requested behavior and tests, not style. Reply with exactly PASS or FAIL on the first line, then one concrete reason on the second line. Do not invent failures.

Request:
Build a tiny Rust tic-tac-toe game. Support legal moves, reject occupied cells, detect row, column, and diagonal wins, detect a draw, and include unit tests for those behaviors.


Plan:
- **Board + move API**: Implement a 3×3 board with `make_move(row, col, player)` returning `Ok`/`Err`; reject moves on occupied cells and out-of-range indices.
- **Win detection**: After each move, check the placed player's row, column, and both diagonals for three-in-a-row; return the winner.
- **Draw detection**: If the board is full and no winner exists, report a draw.
- **Game loop / CLI**: Simple `main` that alternates X and O, prints the board, prompts for coordinates, and handles invalid input by re-prompting.
- **Unit tests**: Cover legal moves, occupied-cell rejection, each win type (row, col, both diagonals), and a draw.

*Success criteria*: `cargo test` passes all above cases; `cargo run` plays a full game where illegal moves are rejected and terminal states (win/draw) end the loop.

Implementation:
```rust
use std::fmt;
use std::io::{self, Write};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub fn other(self) -> Player {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Player::X => write!(f, "X"),
            Player::O => write!(f, "O"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameState {
    InProgress,
    Win(Player),
    Draw,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Board {
    cells: [[Option<Player>; 3]; 3],
}

impl Board {
    pub fn new() -> Self {
        Board::default()
    }

    pub fn get(&self, row: usize, col: usize) -> Option<Player> {
        self.cells[row][col]
    }

    pub fn make_move(
        &mut self,
        row: usize,
        col: usize,
        player: Player,
    ) -> Result<(), &'static str> {
        if row >= 3 || col >= 3 {
            return Err("move is out of range");
        }
        if self.cells[row][col].is_some() {
            return Err("cell is already occupied");
        }
        self.cells[row][col] = Some(player);
        Ok(())
    }

    fn is_full(&self) -> bool {
        self.cells.iter().all(|r| r.iter().all(|c| c.is_some()))
    }

    fn check_win(&self, player: Player) -> bool {
        let p = Some(player);
        for i in 0..3 {
            if (0..3).all(|j| self.cells[i][j] == p) {
                return true;
            }
            if (0..3).all(|j| self.cells[j][i] == p) {
                return true;
            }
        }
        if (0..3).all(|i| self.cells[i][i] == p) {
            return true;
        }
        if (0..3).all(|i| self.cells[i][2 - i] == p) {
            return true;
        }
        false
    }

    pub fn state(&self) -> GameState {
        if self.check_win(Player::X) {
            GameState::Win(Player::X)
        } else if self.check_win(Player::O) {
            GameState::Win(Player::O)
        } else if self.is_full() {
            GameState::Draw
        } else {
            GameState::InProgress
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..3 {
            for col in 0..3 {
                let ch = match self.cells[row][col] {
                    Some(Player::X) => 'X',
                    Some(Player::O) => 'O',
                    None => ' ',
                };
                write!(f, " {} ", ch)?;
                if col < 2 {
                    write!(f, "|")?;
                }
            }
            writeln!(f)?;
            if row < 2 {
                writeln!(f, "---+---+---")?;
            }
        }
        Ok(())
    }
}

fn parse_coord(s: &str) -> Option<usize> {
    match s.parse::<usize>() {
        Ok(v) if v < 3 => Some(v),
        _ => None,
    }
}

fn main() {
    let mut board = Board::new();
    let mut player = Player::X;

    loop {
        println!("{}", board);
        match board.state() {
            GameState::Win(p) => {
                println!("Player {} wins!", p);
                break;
            }
            GameState::Draw => {
                println!("It's a draw!");
                break;
            }
            GameState::InProgress => {}
        }

        print!("Player {} - enter row col (0-2): ", player);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input; try again.");
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 2 {
            println!("Please enter two numbers separated by space.");
            continue;
        }

        let row = match parse_coord(parts[0]) {
            Some(r) => r,
            None => {
                println!("Invalid row. Use 0, 1, or 2.");
                continue;
            }
        };
        let col = match parse_coord(parts[1]) {
            Some(c) => c,
            None => {
                println!("Invalid col. Use 0, 1, or 2.");
                continue;
            }
        };

        match board.make_move(row, col, player) {
            Ok(()) => {
                player = player.other();
            }
            Err(e) => {
                println!("Illegal move: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_move_places_piece() {
        let mut b = Board::new();
        assert_eq!(b.make_move(0, 0, Player::X), Ok(()));
        assert_eq!(b.get(0, 0), Some(Player::X));
        assert_eq!(b.make_move(1, 1, Player::O), Ok(()));
        assert_eq!(b.get(1, 1), Some(Player::O));
    }

    #[test]
    fn rejects_occupied_cell() {
        let mut b = Board::new();
        b.make_move(0, 0, Player::X).unwrap();
        assert!(b.make_move(0, 0, Player::O).is_err());
        assert_eq!(b.get(0, 0), Some(Player::X));
    }

    #[test]
    fn rejects_out_of_range() {
        let mut b = Board::new();
        assert!(b.make_move(3, 0, Player::X).is_err());
        assert!(b.make_move(0, 3, Player::X).is_err());
        assert!(b.make_move(5, 5, Player::O).is_err());
    }

    #[test]
    fn win_by_row() {
        let mut b = Board::new();
        b.make_move(1, 0, Player::X).unwrap();
        b.make_move(1, 1, Player::X).unwrap();
        b.make_move(1, 2, Player::X).unwrap();
        assert_eq!(b.state(), GameState::Win(Player::X));
    }

    #[test]
    fn win_by_column() {
        let mut b = Board::new();
        b.make_move(0, 2, Player::O).unwrap();
        b.make_move(1, 2, Player::O).unwrap();
        b.make_move(2, 2, Player::O).unwrap();
        assert_eq!(b.state(), GameState::Win(Player::O));
    }

    #[test]
    fn win_by_main_diagonal() {
        let mut b = Board::new();
        b.make_move(0, 0, Player::X).unwrap();
        b.make_move(1, 1, Player::X).unwrap();
        b.make_move(2, 2, Player::X).unwrap();
        assert_eq!(b.state(), GameState::Win(Player::X));
    }

    #[test]
    fn win_by_anti_diagonal() {
        let mut b = Board::new();
        b.make_move(0, 2, Player::O).unwrap();
        b.make_move(1, 1, Player::O).unwrap();
        b.make_move(2, 0, Player::O).unwrap();
        assert_eq!(b.state(), GameState::Win(Player::O));
    }

    #[test]
    fn detect_draw() {
        let mut b = Board::new();
        // X O X
        // X O O
        // O X X
        b.make_move(0, 0, Player::X).unwrap();
        b.make_move(0, 1, Player::O).unwrap();
        b.make_move(0, 2, Player::X).unwrap();
        b.make_move(1, 0, Player::X).unwrap();
        b.make_move(1, 1, Player::O).unwrap();
        b.make_move(1, 2, Player::O).unwrap();
        b.make_move(2, 0, Player::O).unwrap();
        b.make_move(2, 1, Player::X).unwrap();
        b.make_move(2, 2, Player::X).unwrap();
        assert_eq!(b.state(), GameState::Draw);
    }

    #[test]
    fn in_progress_initially() {
        let b = Board::new();
        assert_eq!(b.state(), GameState::InProgress);
    }

    #[test]
    fn in_progress_after_partial_board() {
        let mut b = Board::new();
        b.make_move(0, 0, Player::X).unwrap();
        b.make_move(1, 1, Player::O).unwrap();
        assert_eq!(b.state(), GameState::InProgress);
    }
}
```
