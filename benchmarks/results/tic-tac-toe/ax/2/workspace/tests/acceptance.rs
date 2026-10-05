use generated_example::play;

fn empty() -> [[char; 3]; 3] { [['.'; 3]; 3] }

#[test]
fn legal_move_and_no_winner() {
    let mut board = empty();
    assert_eq!(play(&mut board, 1, 2, 'X').unwrap(), None);
    assert_eq!(board[1][2], 'X');
}

#[test]
fn invalid_moves_do_not_mutate_board() {
    let mut board = empty();
    let before = board;
    assert!(play(&mut board, 9, 0, 'X').is_err());
    assert!(play(&mut board, 0, 0, 'Z').is_err());
    assert_eq!(board, before);
    play(&mut board, 0, 0, 'O').unwrap();
    let before = board;
    assert!(play(&mut board, 0, 0, 'X').is_err());
    assert_eq!(board, before);
}

#[test]
fn detects_rows_columns_and_diagonals() {
    let mut row = [['X', 'X', '.'], ['.', '.', '.'], ['.', '.', '.']];
    assert_eq!(play(&mut row, 0, 2, 'X').unwrap(), Some('X'));
    let mut col = [['O', '.', '.'], ['O', '.', '.'], ['.', '.', '.']];
    assert_eq!(play(&mut col, 2, 0, 'O').unwrap(), Some('O'));
    let mut main = [['X', '.', '.'], ['.', 'X', '.'], ['.', '.', '.']];
    assert_eq!(play(&mut main, 2, 2, 'X').unwrap(), Some('X'));
    let mut anti = [['.', '.', 'O'], ['.', 'O', '.'], ['.', '.', '.']];
    assert_eq!(play(&mut anti, 2, 0, 'O').unwrap(), Some('O'));
}

#[test]
fn draw_is_not_a_win() {
    let mut board = [['X', 'O', 'X'], ['X', 'O', 'O'], ['O', 'X', '.']];
    assert_eq!(play(&mut board, 2, 2, 'X').unwrap(), Some('D'));
}
