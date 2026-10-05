//! A tiny tic-tac-toe engine.
//!
//! The crate exposes a 3x3 [`Board`], a [`Player`] enum, a [`Game`] driver that
//! enforces legal moves, and a [`GameState`] describing whether play is ongoing,
//! won by someone, or drawn.
//!
//! ```
//! use generated_example::{Game, GameState, Player};
//!
//! let mut game = Game::new();
//! game.play(0, 0).unwrap();
//! game.play(1, 0).unwrap();
//! game.play(0, 1).unwrap();
//! game.play(1, 1).unwrap();
//! assert_eq!(game.play(0, 2).unwrap(), GameState::Win(Player::X));
//! ```

use std::fmt;

/// Size of the square board (3x3).
pub const BOARD_SIZE: usize = 3;

/// A tic-tac-toe player / mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Player {
    X,
    O,
}

impl Player {
    /// The player who moves next.
    pub fn other(self) -> Player {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }

    /// Single-character rendering of the mark.
    pub fn glyph(self) -> char {
        match self {
            Player::X => 'X',
            Player::O => 'O',
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.glyph())
    }
}

/// Reason a move was rejected. Rejected moves never mutate the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// Row or column was not in `0..3`.
    OutOfBounds { row: usize, col: usize },
    /// The target cell already holds a mark.
    Occupied { row: usize, col: usize },
    /// The game already ended in a win or a draw.
    GameOver(GameState),
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveError::OutOfBounds { row, col } => {
                write!(f, "position ({row}, {col}) is outside the 3x3 board")
            }
            MoveError::Occupied { row, col } => {
                write!(f, "cell ({row}, {col}) is already occupied")
            }
            MoveError::GameOver(state) => write!(f, "game is over ({state})"),
        }
    }
}

impl std::error::Error for MoveError {}

/// Current status of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// Moves remain and nobody has won.
    InProgress,
    /// `Player` completed a row, column, or diagonal.
    Win(Player),
    /// All nine cells are filled with no winning line.
    Draw,
}

impl GameState {
    /// True once the game can no longer change.
    pub fn is_finished(self) -> bool {
        !matches!(self, GameState::InProgress)
    }

    /// The winning player, if any.
    pub fn winner(self) -> Option<Player> {
        match self {
            GameState::Win(player) => Some(player),
            _ => None,
        }
    }
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameState::InProgress => write!(f, "in progress"),
            GameState::Win(player) => write!(f, "{player} wins"),
            GameState::Draw => write!(f, "draw"),
        }
    }
}

/// A 3x3 grid of optional marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    cells: [[Option<Player>; BOARD_SIZE]; BOARD_SIZE],
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

impl Board {
    /// An empty board.
    pub fn new() -> Self {
        Board {
            cells: [[None; BOARD_SIZE]; BOARD_SIZE],
        }
    }

    /// Is `(row, col)` inside the board?
    pub fn is_in_bounds(row: usize, col: usize) -> bool {
        row < BOARD_SIZE && col < BOARD_SIZE
    }

    /// The mark at `(row, col)`, or `None` for empty or out-of-bounds cells.
    pub fn get(&self, row: usize, col: usize) -> Option<Player> {
        if !Board::is_in_bounds(row, col) {
            return None;
        }
        self.cells[row][col]
    }

    /// Place `player` in `(row, col)`.
    ///
    /// Returns `None` on success, or the rejection reason when the cell is
    /// out of bounds or already occupied. The board is left untouched on error.
    pub fn place(&mut self, row: usize, col: usize, player: Player) -> Option<MoveError> {
        if !Board::is_in_bounds(row, col) {
            return Some(MoveError::OutOfBounds { row, col });
        }
        if self.cells[row][col].is_some() {
            return Some(MoveError::Occupied { row, col });
        }
        self.cells[row][col] = Some(player);
        None
    }

    /// Number of occupied cells.
    pub fn filled_count(&self) -> usize {
        self.cells
            .iter()
            .flatten()
            .filter(|cell| cell.is_some())
            .count()
    }

    /// True when every cell is occupied.
    pub fn is_full(&self) -> bool {
        self.filled_count() == BOARD_SIZE * BOARD_SIZE
    }

    /// Every line (rows, columns, and both diagonals) as coordinate triples.
    pub fn lines() -> [[(usize, usize); BOARD_SIZE]; 8] {
        [
            // Rows.
            [(0, 0), (0, 1), (0, 2)],
            [(1, 0), (1, 1), (1, 2)],
            [(2, 0), (2, 1), (2, 2)],
            // Columns.
            [(0, 0), (1, 0), (2, 0)],
            [(0, 1), (1, 1), (2, 1)],
            [(0, 2), (1, 2), (2, 2)],
            // Diagonals.
            [(0, 0), (1, 1), (2, 2)],
            [(0, 2), (1, 1), (2, 0)],
        ]
    }

    /// The winning line and its player, if any line is complete.
    pub fn winning_line(&self) -> Option<(Player, [(usize, usize); BOARD_SIZE])> {
        for line in Board::lines() {
            let Some(player) = self.get(line[0].0, line[0].1) else {
                continue;
            };
            if line[1..]
                .iter()
                .all(|&(r, c)| self.get(r, c) == Some(player))
            {
                return Some((player, line));
            }
        }
        None
    }

    /// Board status given the current marks.
    pub fn state(&self) -> GameState {
        if let Some((player, _)) = self.winning_line() {
            GameState::Win(player)
        } else if self.is_full() {
            GameState::Draw
        } else {
            GameState::InProgress
        }
    }

    /// A tiny ASCII rendering, handy for a CLI or debugging.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                let glyph = self.get(row, col).map_or('.', Player::glyph);
                out.push(glyph);
                if col + 1 < BOARD_SIZE {
                    out.push(' ');
                }
            }
            out.push('\n');
        }
        out
    }
}

/// A game in progress: board plus whose turn it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Game {
    board: Board,
    current: Player,
    state: GameState,
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

impl Game {
    /// A fresh game with `X` to move.
    pub fn new() -> Self {
        Game {
            board: Board::new(),
            current: Player::X,
            state: GameState::InProgress,
        }
    }

    /// The board as it currently stands.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Whose turn it is. After a finished game this is the player who would
    /// have moved next.
    pub fn current_player(&self) -> Player {
        self.current
    }

    /// Current status.
    pub fn state(&self) -> GameState {
        self.state
    }

    /// Winner, if the game has been won.
    pub fn winner(&self) -> Option<Player> {
        self.state.winner()
    }

    /// Play `(row, col)` for the current player.
    ///
    /// Rejects out-of-bounds cells, occupied cells, and moves after the game
    /// ended — in every case the game is left unchanged. On success the turn
    /// passes (unless the game just ended) and the new [`GameState`] is
    /// returned.
    pub fn play(&mut self, row: usize, col: usize) -> Result<GameState, MoveError> {
        if self.state.is_finished() {
            return Err(MoveError::GameOver(self.state));
        }
        // Validate against a copy so a rejected move cannot mutate anything.
        let mut next = self.board;
        if let Some(err) = next.place(row, col, self.current) {
            return Err(err);
        }
        let state = next.state();
        self.board = next;
        self.state = state;
        if !state.is_finished() {
            self.current = self.current.other();
        }
        Ok(state)
    }

    /// A tiny ASCII rendering of the board.
    pub fn render(&self) -> String {
        self.board.render()
    }
}

/// A short scripted game used by several tests: alternates `X`/`O` over the
/// supplied coordinates and returns the game plus every observed state.
#[cfg(test)]
fn play_script(moves: &[(usize, usize)]) -> (Game, Vec<Result<GameState, MoveError>>) {
    let mut game = Game::new();
    let states = moves.iter().map(|&(r, c)| game.play(r, c)).collect();
    (game, states)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A move order whose completed board is a draw:
    /// ```text
    /// X O X
    /// X O O
    /// O X X
    /// ```
    const DRAW_MOVES: [(usize, usize); 9] = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 1),
        (1, 0),
        (1, 2),
        (2, 1),
        (2, 0),
        (2, 2),
    ];

    #[test]
    fn new_game_is_empty_and_x_moves_first() {
        let game = Game::new();
        assert_eq!(game.state(), GameState::InProgress);
        assert_eq!(game.current_player(), Player::X);
        assert_eq!(game.board().filled_count(), 0);
        assert!(!game.board().is_full());
        assert!(game.winner().is_none());
    }

    #[test]
    fn legal_move_updates_board_and_swaps_turn() {
        let mut game = Game::new();
        assert_eq!(game.play(1, 1), Ok(GameState::InProgress));
        assert_eq!(game.board().get(1, 1), Some(Player::X));
        assert_eq!(game.current_player(), Player::O);

        assert_eq!(game.play(0, 1), Ok(GameState::InProgress));
        assert_eq!(game.board().get(0, 1), Some(Player::O));
        assert_eq!(game.current_player(), Player::X);
        assert_eq!(game.board().filled_count(), 2);
    }

    #[test]
    fn occupied_cell_is_rejected_without_mutation() {
        let mut game = Game::new();
        game.play(0, 0).unwrap();
        let snapshot = *game.board();

        assert_eq!(game.play(0, 0), Err(MoveError::Occupied { row: 0, col: 0 }));
        // No mutation: same marks, same turn, still in progress.
        assert_eq!(*game.board(), snapshot);
        assert_eq!(game.board().get(0, 0), Some(Player::X));
        assert_eq!(game.current_player(), Player::O);
        assert_eq!(game.state(), GameState::InProgress);
    }

    #[test]
    fn occupied_cells_are_rejected_in_the_middle_of_a_game() {
        // Fill four cells without completing a line, then poke each of them.
        // X holds (0,0) and (1,1); O holds (0,1) and (1,2).
        let taken = [(0, 0), (0, 1), (1, 1), (1, 2)];
        let (mut game, states) = play_script(&taken);
        assert!(states.iter().all(|s| *s == Ok(GameState::InProgress)));
        for &(r, c) in &taken {
            let snapshot = *game.board();
            assert_eq!(game.play(r, c), Err(MoveError::Occupied { row: r, col: c }));
            assert_eq!(*game.board(), snapshot, "({r}, {c}) must not mutate");
            assert_eq!(game.current_player(), Player::X, "turn must survive");
        }
        // An empty cell that completes no line is still legal: (2,0) gives
        // X (0,0), (1,1), (2,0), which is not a line.
        assert_eq!(game.play(2, 0), Ok(GameState::InProgress));
        assert_eq!(game.board().get(2, 0), Some(Player::X));
    }

    #[test]
    fn every_occupied_cell_is_rejected() {
        let mut board = Board::new();
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                assert_eq!(board.place(row, col, Player::X), None);
            }
        }
        assert!(board.is_full());
        assert_eq!(board.filled_count(), BOARD_SIZE * BOARD_SIZE);
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                assert_eq!(
                    board.place(row, col, Player::O),
                    Some(MoveError::Occupied { row, col })
                );
            }
        }
    }

    #[test]
    fn out_of_bounds_moves_are_rejected_without_mutation() {
        let mut game = Game::new();
        for &(r, c) in &[(3, 0), (0, 3), (3, 3), (9, 9), (0, 100)] {
            let snapshot = *game.board();
            assert_eq!(
                game.play(r, c),
                Err(MoveError::OutOfBounds { row: r, col: c }),
                "({r}, {c}) is off the board"
            );
            assert_eq!(*game.board(), snapshot);
            assert_eq!(game.current_player(), Player::X, "turn must not be lost");
        }
        assert_eq!(game.board().filled_count(), 0);
    }

    #[test]
    fn row_win_is_detected() {
        let mut game = Game::new();
        // X takes row 1: (1,0), (1,1), (1,2); O takes the top row meanwhile.
        game.play(1, 0).unwrap();
        game.play(0, 0).unwrap();
        game.play(1, 1).unwrap();
        game.play(0, 1).unwrap();
        assert_eq!(game.play(1, 2), Ok(GameState::Win(Player::X)));
        assert_eq!(game.winner(), Some(Player::X));
        assert_eq!(game.state(), GameState::Win(Player::X));
        let (winner, line) = game.board().winning_line().unwrap();
        assert_eq!(winner, Player::X);
        assert_eq!(line, [(1, 0), (1, 1), (1, 2)]);
    }

    #[test]
    fn each_row_can_win() {
        for row in 0..BOARD_SIZE {
            let mut game = Game::new();
            let other_row = (row + 1) % BOARD_SIZE;
            for col in 0..BOARD_SIZE {
                game.play(row, col).unwrap();
                if col + 1 < BOARD_SIZE {
                    // Filler for O: two cells in a different row never win.
                    game.play(other_row, col).unwrap();
                }
            }
            assert_eq!(game.state(), GameState::Win(Player::X), "row {row}");
            assert_eq!(game.board().winning_line().unwrap().1[0].0, row);
        }
    }

    #[test]
    fn column_win_is_detected() {
        let mut game = Game::new();
        // X takes column 2: (0,2), (1,2), (2,2); O plays the left column.
        game.play(0, 2).unwrap();
        game.play(0, 0).unwrap();
        game.play(1, 2).unwrap();
        game.play(1, 0).unwrap();
        assert_eq!(game.play(2, 2), Ok(GameState::Win(Player::X)));
        assert_eq!(game.winner(), Some(Player::X));
        let (winner, line) = game.board().winning_line().unwrap();
        assert_eq!(winner, Player::X);
        assert_eq!(line, [(0, 2), (1, 2), (2, 2)]);
    }

    #[test]
    fn each_column_can_win() {
        for col in 0..BOARD_SIZE {
            let mut game = Game::new();
            let other_col = (col + 1) % BOARD_SIZE;
            for row in 0..BOARD_SIZE {
                game.play(row, col).unwrap();
                if row + 1 < BOARD_SIZE {
                    // Filler for O: two cells in a different column never win.
                    game.play(row, other_col).unwrap();
                }
            }
            assert_eq!(game.state(), GameState::Win(Player::X), "column {col}");
            assert_eq!(game.board().winning_line().unwrap().1[0].1, col);
        }
    }

    #[test]
    fn main_diagonal_win_is_detected() {
        let mut game = Game::new();
        game.play(0, 0).unwrap();
        game.play(0, 1).unwrap();
        game.play(1, 1).unwrap();
        game.play(1, 0).unwrap();
        assert_eq!(game.play(2, 2), Ok(GameState::Win(Player::X)));
        let (winner, line) = game.board().winning_line().unwrap();
        assert_eq!(winner, Player::X);
        assert_eq!(line, [(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn anti_diagonal_win_is_detected() {
        let mut game = Game::new();
        game.play(0, 2).unwrap();
        game.play(0, 0).unwrap();
        game.play(1, 1).unwrap();
        game.play(0, 1).unwrap();
        assert_eq!(game.play(2, 0), Ok(GameState::Win(Player::X)));
        let (winner, line) = game.board().winning_line().unwrap();
        assert_eq!(winner, Player::X);
        assert_eq!(line, [(0, 2), (1, 1), (2, 0)]);
    }

    #[test]
    fn o_can_win_too() {
        let mut game = Game::new();
        // O takes row 2 while X scatters.
        game.play(0, 0).unwrap();
        game.play(2, 0).unwrap();
        game.play(0, 1).unwrap();
        game.play(2, 1).unwrap();
        game.play(1, 0).unwrap();
        assert_eq!(game.play(2, 2), Ok(GameState::Win(Player::O)));
        assert_eq!(game.winner(), Some(Player::O));
        assert_eq!(game.state(), GameState::Win(Player::O));
    }

    #[test]
    fn a_win_is_reported_as_soon_as_it_happens() {
        // X completes column 0 on move 5; the state flips immediately.
        let (game, states) = play_script(&[(0, 0), (0, 1), (1, 0), (1, 1), (2, 0)]);
        assert_eq!(states[..4], [Ok(GameState::InProgress); 4]);
        assert_eq!(states[4], Ok(GameState::Win(Player::X)));
        assert_eq!(game.state(), GameState::Win(Player::X));
    }

    #[test]
    fn full_board_without_a_line_is_a_draw() {
        let (game, states) = play_script(&DRAW_MOVES);
        for state in &states[..DRAW_MOVES.len() - 1] {
            assert_eq!(*state, Ok(GameState::InProgress), "no draw before full");
        }
        assert!(game.board().is_full());
        assert_eq!(
            states[DRAW_MOVES.len() - 1],
            Ok(GameState::Draw),
            "ninth move should draw"
        );
        assert!(game.board().winning_line().is_none());
        assert_eq!(game.state(), GameState::Draw);
        assert_eq!(game.winner(), None);
        assert_eq!(game.render(), "X O X\nX O O\nO X X\n");
    }

    #[test]
    fn no_false_draw_with_eight_cells_filled() {
        let (game, states) = play_script(&DRAW_MOVES[..8]);
        assert!(states.iter().all(|s| *s == Ok(GameState::InProgress)));
        assert_eq!(game.board().filled_count(), 8);
        assert!(!game.board().is_full());
        assert_eq!(game.state(), GameState::InProgress);
        assert_eq!(game.winner(), None);
    }

    #[test]
    fn moves_after_the_game_ends_are_rejected() {
        let (mut game, _) = play_script(&[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)]);
        assert_eq!(game.state(), GameState::Win(Player::X));

        let snapshot = *game.board();
        assert_eq!(
            game.play(2, 2),
            Err(MoveError::GameOver(GameState::Win(Player::X)))
        );
        assert_eq!(*game.board(), snapshot);
        assert_eq!(game.state(), GameState::Win(Player::X));
        // An occupied cell after the game still reports the game as over.
        assert_eq!(
            game.play(0, 0),
            Err(MoveError::GameOver(GameState::Win(Player::X)))
        );
        assert_eq!(game.current_player(), Player::X, "no turn change after a win");
    }

    #[test]
    fn draw_also_blocks_further_moves() {
        let (mut game, _) = play_script(&DRAW_MOVES);
        assert_eq!(game.state(), GameState::Draw);
        assert_eq!(game.play(0, 0), Err(MoveError::GameOver(GameState::Draw)));
    }

    #[test]
    fn board_place_reports_errors_and_leaves_board_clean() {
        let mut board = Board::new();
        assert_eq!(board.place(0, 0, Player::X), None);
        assert_eq!(
            board.place(0, 0, Player::O),
            Some(MoveError::Occupied { row: 0, col: 0 })
        );
        assert_eq!(
            board.place(3, 3, Player::O),
            Some(MoveError::OutOfBounds { row: 3, col: 3 })
        );
        assert_eq!(board.get(0, 0), Some(Player::X));
        assert_eq!(board.filled_count(), 1);
        assert_eq!(board.get(3, 3), None);
        assert!(!Board::is_in_bounds(3, 0));
        assert!(Board::is_in_bounds(2, 2));
    }

    #[test]
    fn board_lines_cover_rows_columns_and_diagonals() {
        let lines = Board::lines();
        assert_eq!(lines.len(), 8);
        // Rows.
        assert_eq!(lines[0], [(0, 0), (0, 1), (0, 2)]);
        assert_eq!(lines[2], [(2, 0), (2, 1), (2, 2)]);
        // Columns.
        assert_eq!(lines[3], [(0, 0), (1, 0), (2, 0)]);
        assert_eq!(lines[5], [(0, 2), (1, 2), (2, 2)]);
        // Diagonals.
        assert_eq!(lines[6], [(0, 0), (1, 1), (2, 2)]);
        assert_eq!(lines[7], [(0, 2), (1, 1), (2, 0)]);
    }

    #[test]
    fn empty_board_has_no_winning_line() {
        let board = Board::new();
        assert!(board.winning_line().is_none());
        assert_eq!(board.state(), GameState::InProgress);
        assert_eq!(board, Board::default());
    }

    #[test]
    fn player_and_state_helpers() {
        assert_eq!(Player::X.other(), Player::O);
        assert_eq!(Player::O.other(), Player::X);
        assert_eq!(Player::X.to_string(), "X");
        assert_eq!(Player::O.glyph(), 'O');
        assert_eq!(Player::O.to_string(), "O");

        assert!(!GameState::InProgress.is_finished());
        assert!(GameState::Draw.is_finished());
        assert!(GameState::Win(Player::O).is_finished());
        assert_eq!(GameState::Win(Player::O).winner(), Some(Player::O));
        assert_eq!(GameState::Draw.winner(), None);
        assert_eq!(GameState::InProgress.winner(), None);
        assert_eq!(GameState::InProgress.to_string(), "in progress");
        assert_eq!(GameState::Draw.to_string(), "draw");
        assert_eq!(GameState::Win(Player::X).to_string(), "X wins");
    }

    #[test]
    fn errors_have_readable_messages() {
        assert_eq!(
            MoveError::Occupied { row: 1, col: 2 }.to_string(),
            "cell (1, 2) is already occupied"
        );
        assert_eq!(
            MoveError::OutOfBounds { row: 4, col: 0 }.to_string(),
            "position (4, 0) is outside the 3x3 board"
        );
        assert_eq!(
            MoveError::GameOver(GameState::Draw).to_string(),
            "game is over (draw)"
        );
    }

    #[test]
    fn render_shows_marks_and_empty_cells() {
        let mut game = Game::new();
        assert_eq!(game.render(), ". . .\n. . .\n. . .\n");
        game.play(0, 0).unwrap();
        game.play(1, 1).unwrap();
        assert_eq!(game.render(), "X . .\n. O .\n. . .\n");
    }
}
