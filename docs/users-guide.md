# game-player User's Guide

`game-player` provides the building blocks for writing a computer player for a two-person game. This guide explains what the
crate does, how to decide which search algorithm to use, how the minimax search works, and how to put it all together into a
working player, using tic-tac-toe as an example.

## Contents

- [Overview](#overview)
- [Choosing a Search Algorithm](#choosing-a-search-algorithm)
- [The Minimax Algorithm](#the-minimax-algorithm)
- [Example: A Tic-Tac-Toe Player Using Minimax](#example-a-tic-tac-toe-player-using-minimax)

## Overview

Playing a game well comes down to answering one question over and over: *given the current position, which move should I make?*
This crate answers that question with game tree search. It explores the moves available in a position, the opponent's possible
replies, the replies to those replies, and so on, and then chooses the move that leads to the best reachable outcome.

The search algorithms themselves are game-independent. What makes them play *your* game is a set of traits that you implement
to describe the game's rules:

| You provide | Trait | Purpose |
| --- | --- | --- |
| A game state | `State` (in `game_player::state`) | Represents a position; knows whose turn it is, whether the game is over, and how to apply a move to produce the next position. Also provides a `fingerprint()` hash used for caching. |
| A position evaluator | `StaticEvaluator` (in `game_player::static_evaluator`) | Assigns a numeric value to a position without any lookahead — the search's notion of "how good is this position?" |
| A move generator | `ResponseGenerator` (in `game_player::minimax`) | Lists every legal move available in a position. |

With those three pieces in place, a single function call runs the search:

```rust
use game_player::minimax::search;

if let Some(action) = search(&evaluator, &response_generator, &state, max_depth) {
    let next_state = state.apply(&action);
    // make the move...
}
```

`search` returns the best action for the player whose turn it is, or `None` if that player has no legal responses.

### The players: Alice and Bob

The crate names the two players **Alice** (`PlayerId::Alice`) and **Bob** (`PlayerId::Bob`). The names are arbitrary, but the
roles are not:

- **Alice is the maximizing player** — higher evaluation values are better for her.
- **Bob is the minimizing player** — lower evaluation values are better for him.

Which of your game's players is "Alice" is up to you (e.g., White in chess, X in tic-tac-toe); just be consistent.

### The evaluator's invariant

`StaticEvaluator::evaluate` must **always** return the value of a position *from Alice's perspective*, regardless of whose turn
it is: higher means better for Alice, lower means better for Bob. Do not flip the sign based on the player to move — the
search's max/min logic depends on this convention, and violating it makes the search choose wrong moves.

Values must lie within `[bob_wins_value(), alice_wins_value()]`. A position where Alice has won must evaluate to
`alice_wins_value()`, and one where Bob has won must evaluate to `bob_wins_value()`.

### What the search assumes

The minimax search assumes a **two-player, zero-sum game with perfect information** and alternating turns: what is good for one
player is exactly as bad for the other, and both players can see the whole position. Classic examples are tic-tac-toe,
checkers, chess, reversi, and connect-four.

## Choosing a Search Algorithm

The crate's design anticipates two search algorithms: **minimax with alpha-beta pruning** (implemented today) and **Monte Carlo
Tree Search (MCTS)** (planned — see the roadmap in the README). Even though only minimax is currently available, it is worth
understanding the trade-offs so you can structure your player accordingly.

Choose **minimax** when:

- **You can write a good static evaluation function.** Minimax's play is only as strong as its evaluator. If you can compute a
  meaningful score for an arbitrary mid-game position (material count, mobility, territory, etc.), minimax will exploit it.
- **The branching factor is modest.** Minimax explores the tree exhaustively (minus pruning), so its cost grows exponentially
  with the number of legal moves per turn. Games with tens of moves per position (chess ≈ 35, checkers ≈ 8) are comfortable;
  games with hundreds (Go ≈ 250) are not.
- **The game is tactical.** Minimax finds forced sequences — traps, combinations, forced wins — reliably, because it considers
  *every* line up to its search depth. If missing a short tactic loses the game, exhaustive search is what you want.
- **You want deterministic, explainable play.** Given the same position and depth, minimax always returns the same move, and
  the chosen line can be read directly out of the tree.

Choose **MCTS** (when it becomes available) when:

- **A good evaluation function is hard to write.** MCTS estimates position values statistically from playouts instead of
  requiring a hand-crafted evaluator.
- **The branching factor is large.** MCTS focuses effort on promising branches rather than visiting every one, so it degrades
  gracefully as the move count grows.
- **You want anytime behavior.** MCTS produces a usable answer whenever it is stopped, and the answer improves smoothly with
  more time. Minimax at a fixed depth is closer to all-or-nothing.
- **Strategic, long-horizon judgment matters more than short tactics.** Playout statistics capture long-term consequences that
  a depth-limited minimax may not see.

For small games — tic-tac-toe included — minimax is the clear choice: the tree is tiny, a perfect evaluator for terminal
positions is trivial to write, and the search plays perfectly.

## The Minimax Algorithm

### The core idea

Minimax models a game as alternating turns of two opponents with exactly opposite goals. Starting from the current position, it
builds a tree: the root is the current position, its children are the positions reachable in one move, their children are the
opponent's replies, and so on, down to a chosen depth (measured in *plies* — one ply is one move by one player).

Positions at the bottom of the tree are scored with the static evaluation function. Values then propagate back up the tree:

- At a position where **Alice** is to move, she picks the child with the **maximum** value — that value becomes the position's
  value.
- At a position where **Bob** is to move, he picks the child with the **minimum** value.

The move `search` returns is the child of the root with the best backed-up value for the player to move. The backed-up value
answers the question: "assuming both sides play as well as this search can see, what is the best outcome I can force?"

### Alpha-beta pruning

A full minimax tree grows as *b*^*d* for branching factor *b* and depth *d*, which quickly becomes infeasible.
Alpha-beta pruning skips branches that provably cannot affect the result. The search carries two bounds:

- **alpha** — the best value the maximizing player is already guaranteed elsewhere in the tree.
- **beta** — the best value the minimizing player is already guaranteed.

While evaluating a position's moves, if the value found so far becomes worse for the opponent than something the opponent could
already get by playing differently earlier in the tree, the opponent will never allow this position to be reached — so the
remaining moves need not be examined. Pruning never changes the chosen move; with good move ordering it can reduce the
effective branching factor to roughly √*b*, letting the search go about twice as deep in the same time.

To make cutoffs happen as early as possible, the implementation sorts each position's moves by a preliminary value (from the
transposition table or the static evaluator) before searching them — best-looking moves first.

### The transposition table

Different move orders often lead to the same position (a *transposition*). The search keeps an internal cache — the
transposition table — mapping each position's `fingerprint()` to a previously computed value, so the position is not evaluated
or re-searched again. This is why `State::fingerprint` must be deterministic, collision-resistant, and depend only on the
position (not on the move history that reached it).

Each cached value carries a **quality**: the number of plies searched below the position to produce it. A raw static evaluation
has quality 0; a position fully searched to the depth limit has the highest quality. A cached value replaces recursion only
when its quality is at least what the current search would achieve by recursing; otherwise it is used just as a preliminary
estimate for move ordering, and the search recurses anyway.

One subtlety: when alpha-beta pruning cuts a position off early, the value computed for it is only a bound, not its true value,
so pruned results are deliberately *not* stored in the table.

The table is created fresh inside each call to `search` and discarded when the search returns.

### Choosing a search depth

`max_depth` caps how many plies ahead the search looks. Deeper search plays stronger but costs exponentially more time. Some
guidance:

- For small games, pass a depth that covers the rest of the game (9 for tic-tac-toe) — the search then plays perfectly.
- For larger games, pick the largest depth that stays within your time budget, and prefer even depths ending after the
  *opponent's* reply so the evaluation isn't skewed by one side having just moved (the horizon effect).
- Descent stops early at any position whose value already indicates a win, and at any position for which the response generator
  returns no moves — so make your generator return an empty `Vec` for terminal positions.

## Example: A Tic-Tac-Toe Player Using Minimax

This section builds a complete computer tic-tac-toe player. Alice plays **X** and Bob plays **O**. The pieces we need, in
order: an action type, a state type implementing `State`, an evaluator implementing `StaticEvaluator`, a move generator
implementing `ResponseGenerator`, and a game loop that calls `search`.

### The action

A tic-tac-toe move just names the square to claim (0–8, row-major):

```rust
/// A move: the current player claims square `index` (0..=8, row-major).
#[derive(Clone, Debug)]
struct Placement {
    index: usize,
}
```

### The state

The board holds nine squares and remembers whose turn it is. A few helpers identify wins:

```rust
use game_player::{PlayerId, State};

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
    [0, 1, 2], [3, 4, 5], [6, 7, 8], // rows
    [0, 3, 6], [1, 4, 7], [2, 5, 8], // columns
    [0, 4, 8], [2, 4, 6],            // diagonals
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
            if self.squares[a] != Square::Empty
                && self.squares[a] == self.squares[b]
                && self.squares[b] == self.squares[c]
            {
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
```

Implementing `State` is mostly mechanical. The interesting method is `fingerprint`: it must map every distinct position to a
distinct 64-bit value. Two bits per square plus one bit for the player to move encodes the whole position exactly, so
collisions are impossible:

```rust
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
```

For games with too much state to pack into 64 bits (chess, say), the standard technique is
[Zobrist hashing](https://en.wikipedia.org/wiki/Zobrist_hashing).

### The evaluator

Remember the invariant: the value is **always from Alice's perspective**. Won positions get the extreme values; for unfinished
positions we use a simple heuristic — the number of lines still winnable by Alice minus the number still winnable by Bob. (At
full search depth the heuristic barely matters, since every line ends in a won, lost, or drawn position; it earns its keep at
shallower depths, and for move ordering.)

```rust
use game_player::StaticEvaluator;

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
```

Any win value with magnitude safely above the heuristic's range works; `±100.0` is plenty here.

### The move generator

Every empty square is a legal move — unless the game is already over, in which case there are no responses. Returning an empty
`Vec` for terminal positions is what stops the search (and the game loop below) from playing past the end of the game:

```rust
use game_player::minimax::ResponseGenerator;

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
```

### Playing the game

Tic-tac-toe lasts at most nine plies, so `max_depth = 9` makes the computer play perfectly. Here the computer plays both
sides; in a real application you would alternate `search` with reading the human's move.

```rust
use game_player::minimax::search;

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
```

Because both sides search the full game tree, the game ends in the expected result for perfect play: a draw.

```text
Alice plays square 4
...
.X.
...

Bob plays square 0
O..
.X.
...

...

Draw
```

The complete program is included in the repository as [examples/tic_tac_toe.rs](../examples/tic_tac_toe.rs); run it with
`cargo run --example tic_tac_toe`.

### Where to go from here

To adapt this pattern to your own game:

1. Replace `Placement`/`Board` with your game's action and state, keeping `apply` non-mutating and `fingerprint` a faithful
   position hash (use Zobrist hashing when the state doesn't fit in 64 bits).
2. Write an evaluator that captures what "winning" looks like in your game — material, mobility, territory — always from
   Alice's perspective and always within `[bob_wins_value(), alice_wins_value()]`.
3. Generate all legal moves, including explicit "pass" actions if your game forces passes; return an empty `Vec` only when the
   player truly cannot respond.
4. Tune `max_depth` to your time budget.
