use game_player::minimax::{ResponseGenerator, search};
use game_player::{PlayerId, State, StaticEvaluator};

/// A move: the current player claims square `index` (0..=8, row-major).
#[derive(Clone, Debug)]
struct Placement {
    index: usize,
}

/// The mark in a single square. Alice plays X, Bob plays O.
#[derive(Clone, Copy, PartialEq)]
enum Square {
    Empty,
    X,
    O,
}

#[derive(Clone)]
struct Board {
    squares: [Square; 9],
    next_player: PlayerId,
}

/// The eight lines that win the game.
const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

fn mark_of(player: PlayerId) -> Square {
    match player {
        PlayerId::Alice => Square::X,
        PlayerId::Bob => Square::O,
    }
}

impl Board {
    fn new() -> Self {
        Self {
            squares: [Square::Empty; 9],
            next_player: PlayerId::Alice,
        }
    }

    fn winner(&self) -> Option<PlayerId> {
        for &[a, b, c] in &LINES {
            if self.squares[a] != Square::Empty && self.squares[a] == self.squares[b] && self.squares[b] == self.squares[c] {
                return Some(if self.squares[a] == Square::X {
                    PlayerId::Alice
                } else {
                    PlayerId::Bob
                });
            }
        }
        None
    }

    fn is_full(&self) -> bool {
        self.squares.iter().all(|&s| s != Square::Empty)
    }
}

impl State for Board {
    type Action = Placement;

    fn fingerprint(&self) -> u64 {
        // 2 bits per square plus 1 bit for whose turn it is: unique for every position.
        let mut fp = 0u64;
        for &square in &self.squares {
            fp = (fp << 2)
                | match square {
                    Square::Empty => 0,
                    Square::X => 1,
                    Square::O => 2,
                };
        }
        (fp << 1) | self.next_player as u64
    }

    fn whose_turn(&self) -> PlayerId {
        self.next_player
    }

    fn is_terminal(&self) -> bool {
        self.winner().is_some() || self.is_full()
    }

    fn apply(&self, action: &Placement) -> Self {
        let mut next = self.clone();
        next.squares[action.index] = mark_of(self.next_player);
        next.next_player = self.next_player.other();
        next
    }
}

struct Evaluator;

impl StaticEvaluator for Evaluator {
    type State = Board;

    // Always from Alice's perspective: positive favors Alice (X), negative favors Bob (O).
    fn evaluate(&self, board: &Board) -> f32 {
        match board.winner() {
            Some(PlayerId::Alice) => self.alice_wins_value(),
            Some(PlayerId::Bob) => self.bob_wins_value(),
            None => {
                // Heuristic: lines still winnable by Alice minus lines still winnable by Bob.
                let mut value = 0.0;
                for line in &LINES {
                    let has_x = line.iter().any(|&i| board.squares[i] == Square::X);
                    let has_o = line.iter().any(|&i| board.squares[i] == Square::O);
                    if !has_o {
                        value += 1.0;
                    }
                    if !has_x {
                        value -= 1.0;
                    }
                }
                value
            }
        }
    }

    fn alice_wins_value(&self) -> f32 {
        100.0
    }

    fn bob_wins_value(&self) -> f32 {
        -100.0
    }
}

struct Moves;

impl ResponseGenerator for Moves {
    type State = Board;

    fn generate(&self, board: &Board, _depth: u32) -> Vec<Placement> {
        if board.is_terminal() {
            return Vec::new(); // No responses once the game is over.
        }
        (0..9)
            .filter(|&i| board.squares[i] == Square::Empty)
            .map(|index| Placement { index })
            .collect()
    }
}

fn show(board: &Board) {
    for row in 0..3 {
        let line: String = (0..3)
            .map(|col| match board.squares[row * 3 + col] {
                Square::Empty => '.',
                Square::X => 'X',
                Square::O => 'O',
            })
            .collect();
        println!("{line}");
    }
    println!();
}

fn main() {
    let evaluator = Evaluator;
    let moves = Moves;
    let mut board = Board::new();

    // Play a full game with the computer playing both sides.
    while let Some(placement) = search(&evaluator, &moves, &board, 9) {
        println!("{:?} plays square {}", board.whose_turn(), placement.index);
        board = board.apply(&placement);
        show(&board);
    }

    match board.winner() {
        Some(player) => println!("{player:?} wins"),
        None => println!("Draw"),
    }
}
