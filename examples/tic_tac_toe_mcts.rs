//! End-to-end validation that `mcts::search` is perspective-correct for
//! adversarial play: MCTS plays tic-tac-toe against a perfect minimax
//! opponent (full-depth alpha-beta search) and must never lose, across both
//! expansion modes and a nonzero `initial_value_weight`. Fully deterministic:
//! no RNG is used anywhere in this example.

use game_player::mcts::{self, DEFAULT_EXPLORATION_CONSTANT};
use game_player::minimax;
use game_player::{PlayerId, State, StaticEvaluator};

const ITERATIONS: u32 = 10_000;

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

/// `mcts::ResponseGenerator` — note there is no `depth` parameter, unlike
/// `minimax::ResponseGenerator` below.
struct MctsMoves;

impl mcts::ResponseGenerator for MctsMoves {
    type State = Board;

    fn generate(&self, board: &Board) -> Vec<Placement> {
        if board.is_terminal() {
            return Vec::new();
        }
        (0..9)
            .filter(|&i| board.squares[i] == Square::Empty)
            .map(|index| Placement { index })
            .collect()
    }
}

/// `minimax::ResponseGenerator` — same move list, `depth` unused.
struct MinimaxMoves;

impl minimax::ResponseGenerator for MinimaxMoves {
    type State = Board;

    fn generate(&self, board: &Board, _depth: u32) -> Vec<Placement> {
        if board.is_terminal() {
            return Vec::new();
        }
        (0..9)
            .filter(|&i| board.squares[i] == Square::Empty)
            .map(|index| Placement { index })
            .collect()
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

/// `mcts::ValueEstimator`: value in `[0.0, 1.0]` from the perspective of the
/// CURRENT player (`state.whose_turn()`). Reuses the same lines heuristic as
/// `Evaluator` above, rescaled into `[0, 1]` and flipped for Bob's turn.
struct LinesEstimator;

impl mcts::ValueEstimator for LinesEstimator {
    type State = Board;
    type ResponseGenerator = MctsMoves;

    fn estimate(&self, state: &Board, _rg: &MctsMoves) -> f32 {
        match state.winner() {
            Some(winner) => {
                if winner == state.whose_turn() {
                    1.0
                } else {
                    0.0
                }
            }
            None if state.is_full() => 0.5,
            None => {
                // Alice-perspective score in [-8, 8]: +1 per O-free line, -1 per X-free line.
                let mut score = 0.0;
                for line in &LINES {
                    let has_x = line.iter().any(|&i| state.squares[i] == Square::X);
                    let has_o = line.iter().any(|&i| state.squares[i] == Square::O);
                    if !has_o {
                        score += 1.0;
                    }
                    if !has_x {
                        score -= 1.0;
                    }
                }
                let v01 = (score + 8.0) / 16.0;
                if state.whose_turn() == PlayerId::Alice {
                    v01
                } else {
                    1.0 - v01
                }
            }
        }
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

/// Plays one full game, with `mcts_player` moved by `mcts::search` (under the
/// given knobs) and the other side moved by a perfect (full-depth) minimax
/// search. Returns the winner, or `None` on a draw.
fn play_game(mcts_player: PlayerId, initial_value_weight: f32, estimate_on_expansion: bool, iterations: u32) -> Option<PlayerId> {
    let mcts_moves = MctsMoves;
    let mcts_estimator = LinesEstimator;
    let minimax_evaluator = Evaluator;
    let minimax_moves = MinimaxMoves;

    let mut board = Board::new();
    while !board.is_terminal() {
        let action = if board.whose_turn() == mcts_player {
            mcts::search(
                &board,
                &mcts_moves,
                &mcts_estimator,
                DEFAULT_EXPLORATION_CONSTANT,
                initial_value_weight,
                estimate_on_expansion,
                iterations,
            )
            .expect("a non-terminal tic-tac-toe board always has legal moves")
        } else {
            minimax::search(&minimax_evaluator, &minimax_moves, &board, 9)
                .expect("a non-terminal tic-tac-toe board always has legal moves")
        };
        board = board.apply(&action);
    }
    show(&board);
    board.winner()
}

fn main() {
    let configs: [(f32, bool); 3] = [(0.0, false), (1.0, false), (1.0, true)];
    let sides = [PlayerId::Alice, PlayerId::Bob];

    let mut games_played = 0;
    for &(initial_value_weight, estimate_on_expansion) in &configs {
        for &mcts_player in &sides {
            let winner = play_game(mcts_player, initial_value_weight, estimate_on_expansion, ITERATIONS);
            games_played += 1;
            println!("config(w={initial_value_weight}, eager={estimate_on_expansion}) mcts={mcts_player:?} -> winner={winner:?}");
            assert!(
                winner != Some(mcts_player.other()),
                "MCTS (as {mcts_player:?}) lost to perfect minimax under config(w={initial_value_weight}, eager={estimate_on_expansion})"
            );
        }
    }

    println!("All {games_played} games completed: MCTS never lost to perfect minimax.");
}
