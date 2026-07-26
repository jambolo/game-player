# game-player User's Guide

`game-player` provides the building blocks for writing a computer player for a two-person game. This guide explains what the
crate does, how to decide which of its two search algorithms — alpha-beta minimax and Monte Carlo Tree Search (MCTS) — fits
your game, how each algorithm works, and how to put the pieces together into a working player. Both algorithms get a complete
worked example, using tic-tac-toe as the game.

## Contents

- [Overview](#overview)
  - [The no-legal-moves policy](#the-no-legal-moves-policy)
- [Choosing a Search Algorithm](#choosing-a-search-algorithm)
- [The Minimax Algorithm](#the-minimax-algorithm)
- [The MCTS Algorithm](#the-mcts-algorithm)
  - [The built-in `RandomPlayoutEstimator`](#the-built-in-randomplayoutestimator)
- [Example: A Tic-Tac-Toe Player Using Minimax](#example-a-tic-tac-toe-player-using-minimax)
- [Example: A Tic-Tac-Toe Player Using MCTS](#example-a-tic-tac-toe-player-using-mcts)
- [Where to Go From Here](#where-to-go-from-here)

## Overview

Playing a game well comes down to answering one question over and over: *given the current position, which move should I make?*
This crate answers that question with game tree search. It explores the moves available in a position, the opponent's possible
replies, the replies to those replies, and so on, and then chooses the move that leads to the best reachable outcome.

The search algorithms themselves are game-independent. What makes them play *your* game is a set of traits that you implement
to describe the game's rules. Both algorithms share one trait — `State` (in `game_player::state`), which represents a
position: it knows whose turn it is, whether the game is over, and how to apply a move to produce the next position, and it
provides a `fingerprint()` hash used for caching. Each algorithm then has its own pair of traits:

| Search | Position judgment | Move generation |
| --- | --- | --- |
| Minimax | `StaticEvaluator` (in `game_player::static_evaluator`) — assigns a numeric value to a position without any lookahead | `minimax::ResponseGenerator` — `generate(&self, state, depth)` lists every legal move |
| MCTS | `mcts::ValueEstimator` — estimates a position's value by any strategy, e.g. a random playout or a static evaluation | `mcts::ResponseGenerator` — `generate(&self, state)` lists every legal move (no `depth` argument) |

The two `ResponseGenerator` traits are distinct types with nearly identical jobs; if you implement both searches for one game,
you implement both traits (usually by delegating to one shared move-listing function).

With the pieces in place, a single function call runs either search:

```rust
use game_player::{minimax, mcts};

// Minimax: search the game tree exhaustively to `max_depth` plies.
if let Some(action) = minimax::search(&evaluator, &minimax_moves, &state, max_depth) {
    let next_state = state.apply(&action);
    // make the move...
}

// MCTS: run `max_iterations` iterations of Monte Carlo Tree Search.
if let Some(action) = mcts::search(
    &state,
    &mcts_moves,
    &estimator,
    mcts::DEFAULT_EXPLORATION_CONSTANT,
    mcts::DEFAULT_INITIAL_VALUE_WEIGHT,
    false, // estimate_on_expansion
    max_iterations,
) {
    let next_state = state.apply(&action);
    // make the move...
}
```

Both searches return the best action found for the player whose turn it is, or `None` if `s0` itself is terminal (see
[The no-legal-moves policy](#the-no-legal-moves-policy)).

### The players: Alice and Bob

The crate names the two players **Alice** (`PlayerId::Alice`) and **Bob** (`PlayerId::Bob`). The names are arbitrary, but the
roles are not:

- **Alice is the maximizing player** — higher evaluation values are better for her.
- **Bob is the minimizing player** — lower evaluation values are better for him.

Which of your game's players is "Alice" is up to you (e.g., White in chess, X in tic-tac-toe); just be consistent.

### Two perspective conventions — don't mix them up

The two position-judgment traits use **different value conventions**, and each search depends on its own convention being
honored. Violating either one makes the corresponding search choose wrong moves.

- **`StaticEvaluator::evaluate` (minimax) is always from Alice's perspective**, regardless of whose turn it is: higher means
  better for Alice, lower means better for Bob. Do not flip the sign based on the player to move. Values must lie within
  `[bob_wins_value(), alice_wins_value()]`; a position where Alice has won must evaluate to `alice_wins_value()`, and one
  where Bob has won to `bob_wins_value()`.
- **`ValueEstimator::estimate` (MCTS) is from the perspective of the player to move** (`state.whose_turn()`), and its value
  must lie in `[0.0, 1.0]`: `0.0` means a certain loss for that player, `1.0` a certain win, `0.5` a draw. For terminal
  states the returned value must be the exact outcome. `0.0` and `1.0` are reserved for that certainty — see
  [the `ValueEstimator` contract](#the-valueestimator-contract).

### What the searches assume

Both searches assume a **two-player, zero-sum game with perfect information**: what is good for one player is exactly as bad
for the other, and both players can see the whole position. Classic examples are tic-tac-toe, checkers, chess, reversi, and
connect-four.

### The no-legal-moves policy

Both searches treat an empty result from `generate` as the definitive signal that a position is the end of the game — neither
`minimax::search` nor `mcts::search` ever calls `State::is_terminal()` itself. This means the two must always agree: your
`generate` implementation must return **at least one action** whenever `is_terminal()` is `false`, and **no actions** when
it's `true`.

If the rules of your game force a player to skip a turn — no legal move exists, but play continues — the game has *not*
ended, so `generate` must still return something: add an explicit "pass" action to your `Action` type and apply it as a
no-op move (typically just flipping `whose_turn()`). Reserve an empty `Vec` strictly for positions where `is_terminal()` is
`true`. A forced resignation, by contrast, ends the game immediately — model that by making `is_terminal()` `true` for the
resulting position rather than inventing a "resign" action.

Getting this backwards is silent and easy to miss: a `generate` that returns `[]` for a merely-stuck-but-ongoing position
makes the search treat that position as the final outcome, using whatever static value or estimate it already has for it
instead of searching further. Both `minimax::search` and `mcts::search` run a debug assertion right after every call to
`generate` that panics if its result's emptiness ever disagrees with `is_terminal()`, so a violation surfaces immediately in
a debug or test build rather than degrading search quality silently.

## Choosing a Search Algorithm

The crate provides two search algorithms: **minimax with alpha-beta pruning** and **Monte Carlo Tree Search (MCTS)**. It is
worth understanding the trade-offs between them so you can structure your player accordingly.

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

Choose **MCTS** when:

- **A good evaluation function is hard to write.** MCTS estimates position values with a pluggable estimator — a random
  playout to the end of the game is the classic choice, but any strategy works — instead of requiring a hand-crafted
  evaluator.
- **The branching factor is large.** MCTS focuses effort on promising branches rather than visiting every one, so it degrades
  gracefully as the move count grows.
- **You want anytime behavior.** MCTS produces a usable answer whenever it is stopped, and the answer improves smoothly with
  more time. Minimax at a fixed depth is closer to all-or-nothing.
- **Strategic, long-horizon judgment matters more than short tactics.** Playout statistics capture long-term consequences that
  a depth-limited minimax may not see.

For a game as small as tic-tac-toe, either algorithm plays it well; minimax plays it *perfectly*, because the whole tree fits
within its search depth. This guide builds both players anyway — the small game keeps the code readable — and the MCTS example
ends by pitting the two against each other.

## The Minimax Algorithm

### The core idea of minimax

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
  returns no moves — so make your generator return an empty `Vec` for terminal positions only, and at least one move (e.g. a
  "pass") for every other position (see [The no-legal-moves policy](#the-no-legal-moves-policy)).

## The MCTS Algorithm

### The core idea of MCTS

Where minimax explores every line to a fixed depth, MCTS grows a search tree *incrementally and asymmetrically*: it spends its
budget on the lines that look most promising so far, while still occasionally probing neglected ones. It needs no
Alice-perspective evaluation function — just an *estimator* that can produce a rough value for any position — and it can be
stopped at any time, returning the best answer found so far.

Each call to `mcts::search` runs a fixed number of iterations, and every iteration performs the same four phases:

1. **Selection** — starting at the root, descend the tree by repeatedly moving to the child with the highest UCT score (see
   below), until reaching a node that still has untried moves, has no children, or is terminal.
2. **Expansion** — add a new child to the selected node by applying one of its untried moves.
3. **Evaluation** — call the `ValueEstimator` on the new child's state to get a value in `[0.0, 1.0]`.
4. **Back-propagation** — walk from the new child back up to the root, updating each ancestor's visit count and value sum with
   the result.

After the last iteration, the returned action is the one leading to the **most-visited** child of the root — visit count,
not average value, because it is the more robust statistic: a child only accumulates visits by repeatedly surviving selection.

### The UCT score

Selection balances *exploitation* (revisit the child that has scored well) against *exploration* (try the child we know little
about) with the UCT formula. For a child with `visits` visits, accumulated `value_sum`, and parent visit count `N`:

```text
Q   = value_sum / visits                 (average result so far)
UCT = Q + c * sqrt(ln(N) / visits)
```

The first term favors children that have performed well; the second grows for children visited rarely relative to their
parent. The constant `c` (the `exploration_constant` parameter) sets the balance: higher values explore more. A never-visited
child has an infinite UCT score, so it is always tried before any sibling is revisited.

### The `ValueEstimator` contract

`ValueEstimator::estimate(&self, state, rg) -> f32` is MCTS's counterpart to minimax's `StaticEvaluator`, but its convention is
different — see [the perspective conventions](#two-perspective-conventions--dont-mix-them-up) above. It returns a value in
`[0.0, 1.0]` from the perspective of **the player to move** (`state.whose_turn()`): `0.0` a certain loss for that player,
`1.0` a certain win, `0.5` a draw, and for terminal states the exact outcome.

Any strategy that produces such a value qualifies: a random playout to the end of the game (the classic MCTS rollout, using
the `rg` argument to generate moves), a static evaluation, a neural network. The estimator is pluggable precisely so you can
start with something cheap and swap in something stronger later.

**Reserve `0.0` and `1.0` for certainty.** Those boundary values mean "this outcome is decided," so only return them when it
actually is: a terminal state's exact outcome, or a rollout that played all the way to one. A heuristic that stops short of
the end of the game — a static evaluation, a lookahead cut off at some depth, a neural network's confidence score — is an
opinion, not a proof, and should stay strictly inside `(0.0, 1.0)` even when it's very confident, for example by clamping
just shy of the boundary. Otherwise a merely strong-looking but undecided position becomes indistinguishable, in the
search's accumulated statistics, from one that is actually won or lost.

If you already have an Alice-perspective `StaticEvaluator`, you can adapt it: normalize its output into `[0.0, 1.0]` with
`v01 = (eval - bob_wins_value()) / (alice_wins_value() - bob_wins_value())`, then return `v01` when `whose_turn()` is Alice
and `1.0 - v01` when it is Bob. The MCTS example below does exactly this. This naturally respects the certainty-reservation
above too, as long as the wrapped `StaticEvaluator` respects its own contract of returning `alice_wins_value()` /
`bob_wins_value()` only for an actual win — never as a heuristic's high-confidence guess.

### The built-in `RandomPlayoutEstimator`

Writing the classic rollout estimator by hand is repetitive enough that the crate ships one:
`game_player::random_playout::RandomPlayoutEstimator<G>`. It repeatedly samples a uniformly-random action from
`rg.generate(state)` and applies it until the state is terminal, then reports the result — exactly the "play random legal
moves until the game ends" strategy described above.

It is behind the `mcts_random_playout` feature flag, off by default, because it needs a random number generator and the crate
does not want to force a `rand` dependency onto callers who bring their own estimator (a static evaluation or a neural
network, say — the common case). Enable it in `Cargo.toml`:

```toml
game-player = { version = "...", features = ["mcts_random_playout"] }
```

Two things to know before using it:

- **Your state needs one more trait.** `State` only exposes `is_terminal()` — it has no generic way to say *who* won, which a
  playout needs to know once it reaches the end. So `RandomPlayoutEstimator` requires `G::State` to also implement
  `random_playout::TerminalOutcome`, a single-method trait:

  ```rust
  trait TerminalOutcome: State {
      /// Same contract as `ValueEstimator::estimate` on a terminal state: [0.0, 1.0] from the
      /// perspective of `self.whose_turn()`. Only ever called when `is_terminal()` is true.
      fn outcome(&self) -> f32;
  }
  ```

  For tic-tac-toe this is a small addition to `Board`:

  ```rust
  use game_player::random_playout::TerminalOutcome;

  impl TerminalOutcome for Board {
      fn outcome(&self) -> f32 {
          match self.winner() {
              Some(winner) if winner == self.whose_turn() => 1.0,
              Some(_) => 0.0,
              None => 0.5, // Draw (is_full()).
          }
      }
  }
  ```

- **It carries its own RNG, seeded for reproducibility.** Construct it with `RandomPlayoutEstimator::new(seed)`; the same
  seed and the same sequence of `estimate()` calls always produce the same playouts, so a search built on it stays
  reproducible run to run (as long as it isn't shared across concurrent searches).

Putting it together, in place of a hand-written `LinesEstimator`:

```rust
use game_player::random_playout::RandomPlayoutEstimator;

let estimator = RandomPlayoutEstimator::<MctsMoves>::new(42);
```

The rest of the search call is unchanged — `estimator` still just needs to satisfy `mcts::ValueEstimator`.

### How MCTS stays adversarial

Minimax alternates max and min levels explicitly. MCTS achieves the same thing through bookkeeping: each node's statistics are
stored from the perspective of **the player who chose the move leading into it** (i.e. `whose_turn()` of the *parent's*
state). During back-propagation, each ancestor is credited `value` if its perspective player matches the evaluated leaf's
player to move, and `1.0 - value` otherwise. With every node scored from its chooser's point of view, selection is a plain
argmax of UCT at every level — no max/min alternation needed — and the search remains correct even in games where a player may
move twice in a row (perspectives are compared by `whose_turn()` equality, never by ply parity).

### Tuning knobs

`mcts::search` takes four tuning parameters:

- **`exploration_constant`** (`c`, default `DEFAULT_EXPLORATION_CONSTANT` = √2) — the UCT exploration weight. Higher values
  favor exploring less-visited nodes over exploiting the best-known one.
- **`initial_value_weight`** (`w`, default `DEFAULT_INITIAL_VALUE_WEIGHT` = `0.0`) — how much to trust a node's first
  estimate. Each newly created node stores the estimator's initial opinion of it (`v0`); `w` blends that opinion into UCT as
  if the node had already been visited `w` extra times with that result: `n_eff = visits + w`,
  `Q = (value_sum + w*v0) / n_eff`, `UCT = Q + c * sqrt(ln(N) / n_eff)`. `0.0` disables the blend (pure classic UCT); larger
  values let the estimator's judgment dominate longer before accumulated visit statistics take over.
- **`estimate_on_expansion`** — chooses between lazy and eager expansion. `false` (the default) is lazy: each iteration
  creates and estimates exactly one child, which is the right choice for expensive estimators such as full playouts. `true` is
  eager: every untried child of the expanded node is created and estimated at once (one estimator call each), letting UCT rank
  all of them immediately — worthwhile for cheap estimators such as a static evaluation, wasteful for expensive ones, and
  pointless when `initial_value_weight` is `0.0`, since the stored estimates would then never be used.
- **`max_iterations`** — the search budget. More iterations play stronger, with cost growing linearly; because MCTS is an
  anytime algorithm, there is no "cliff" — pick the largest budget your time allows.

## Example: A Tic-Tac-Toe Player Using Minimax

This section builds a complete computer tic-tac-toe player. Alice plays **X** and Bob plays **O**. The pieces we need, in
order: an action type, a state type implementing `State`, an evaluator implementing `StaticEvaluator`, a move generator
implementing `ResponseGenerator`, and a game loop that calls `search`. (The action, state, and board helpers built here are
shared by the [MCTS example](#example-a-tic-tac-toe-player-using-mcts) that follows.)

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
`Vec` for terminal positions is what stops the search (and the game loop below) from playing past the end of the game; per
[the no-legal-moves policy](#the-no-legal-moves-policy), an empty `Vec` must be reserved for exactly that case. Tic-tac-toe
never has a position where a player is merely stuck with the game still ongoing, so no "pass" action is needed here — a game
that can produce such a position must add one (see [Where to Go From Here](#where-to-go-from-here)):

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

## Example: A Tic-Tac-Toe Player Using MCTS

Now the same game with the other search. The `Placement`, `Square`, `Board`, and `LINES` definitions — everything through the
`State` implementation — carry over unchanged from the minimax example; a state knows nothing about which search is exploring
it. What changes are the two search-facing traits: the move generator and the position judgment. To finish, we'll pit the MCTS
player against the perfect minimax player from the previous section.

### The MCTS move generator

`mcts::ResponseGenerator` is a distinct trait from `minimax::ResponseGenerator` — its `generate` takes no `depth` argument —
but for tic-tac-toe the move list is identical:

```rust
use game_player::mcts;

struct MctsMoves;

impl mcts::ResponseGenerator for MctsMoves {
    type State = Board;

    fn generate(&self, board: &Board) -> Vec<Placement> {
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

### The value estimator

The classic MCTS estimator is a random playout: play random legal moves (via the `rg` argument) until the game ends, and
report the outcome. The crate ships one — see
[The built-in `RandomPlayoutEstimator`](#the-built-in-randomplayoutestimator) — but it lives behind an off-by-default feature
flag and needs an RNG, so here we instead demonstrate adapting the *lines* heuristic from the minimax evaluator, following the
recipe in [the `ValueEstimator` contract](#the-valueestimator-contract): compute the Alice-perspective score, rescale it into
`[0.0, 1.0]`, and flip it when it is Bob's turn. This also makes the example fully deterministic — no RNG anywhere.

Mind the two halves of the contract: terminal states must return the **exact** outcome (`1.0` win / `0.0` loss / `0.5` draw),
and every value is from the perspective of `state.whose_turn()` — not Alice's:

```rust
/// `mcts::ValueEstimator`: value in [0.0, 1.0] from the perspective of the
/// CURRENT player (`state.whose_turn()`). Reuses the lines heuristic from the
/// minimax `Evaluator`, rescaled into [0, 1] and flipped for Bob's turn.
struct LinesEstimator;

impl mcts::ValueEstimator for LinesEstimator {
    type State = Board;
    type ResponseGenerator = MctsMoves;

    fn estimate(&self, state: &Board, _rg: &MctsMoves) -> f32 {
        match state.winner() {
            // Terminal states must report the exact outcome.
            Some(winner) => {
                if winner == state.whose_turn() { 1.0 } else { 0.0 }
            }
            None if state.is_full() => 0.5, // Draw.
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
                // Rescale into [0, 1], then flip perspective when it is Bob's turn.
                let v01 = (score + 8.0) / 16.0;
                if state.whose_turn() == PlayerId::Alice { v01 } else { 1.0 - v01 }
            }
        }
    }
}
```

The (mildly counterintuitive) `winner == state.whose_turn()` check handles both outcomes: in tic-tac-toe the winner is always
the player who just moved, so from the perspective of the player *now* to move a decided game is normally a loss (`0.0`) — but
the check stays correct even for game rules where that assumption fails.

### Playing against the minimax player

The sternest test available: MCTS as X against the perfect minimax player from the previous example as O. Since perfect play
never loses, the best MCTS can achieve is a draw — and with a healthy iteration budget, that is what it achieves:

```rust
use game_player::mcts::DEFAULT_EXPLORATION_CONSTANT;
use game_player::minimax;

const ITERATIONS: u32 = 10_000;

fn main() {
    let mcts_moves = MctsMoves;
    let estimator = LinesEstimator;
    let minimax_evaluator = Evaluator;
    let minimax_moves = Moves; // the minimax::ResponseGenerator from the previous example

    let mut board = Board::new();
    while !board.is_terminal() {
        let placement = if board.whose_turn() == PlayerId::Alice {
            // MCTS plays X.
            mcts::search(
                &board,
                &mcts_moves,
                &estimator,
                DEFAULT_EXPLORATION_CONSTANT,
                0.0,   // initial_value_weight: pure classic UCT
                false, // estimate_on_expansion: lazy
                ITERATIONS,
            )
            .expect("a non-terminal board always has legal moves")
        } else {
            // Perfect minimax plays O.
            minimax::search(&minimax_evaluator, &minimax_moves, &board, 9)
                .expect("a non-terminal board always has legal moves")
        };
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

To experiment with the tuning knobs, try `initial_value_weight = 1.0` (the estimator's first impression of each node counts as
one virtual visit) with `estimate_on_expansion = true` — with an estimator this cheap, eagerly estimating every child on
expansion is affordable and lets UCT rank new children by the heuristic instead of visiting them in arbitrary order. Lowering
`ITERATIONS` is instructive too: watch how the play degrades gracefully rather than falling off a cliff.

The complete program is included in the repository as
[examples/tic_tac_toe_mcts.rs](../examples/tic_tac_toe_mcts.rs); run it with `cargo run --example tic_tac_toe_mcts`. It goes
further than the listing above: it plays MCTS on both sides of the board, across lazy and eager expansion and zero and nonzero
initial-value weights, and asserts that MCTS never loses to perfect play in any configuration.

## Where to Go From Here

To adapt these patterns to your own game:

1. Replace `Placement`/`Board` with your game's action and state, keeping `apply` non-mutating and `fingerprint` a faithful
   position hash (use Zobrist hashing when the state doesn't fit in 64 bits).
2. Generate all legal moves, per [the no-legal-moves policy](#the-no-legal-moves-policy): include an explicit "pass" action
   whenever the rules force a player to skip a turn without ending the game, and return an empty `Vec` only when
   `is_terminal()` is `true`. If you use both searches, implement both `ResponseGenerator` traits over one shared
   move-listing function.
3. For **minimax**: write an evaluator that captures what "winning" looks like in your game — material, mobility, territory —
   always from Alice's perspective and always within `[bob_wins_value(), alice_wins_value()]`; then tune `max_depth` to your
   time budget.
4. For **MCTS**: write an estimator that returns `[0.0, 1.0]` from the current player's perspective — start with a random
   playout (the crate ships one behind the `mcts_random_playout` feature; see
   [The built-in `RandomPlayoutEstimator`](#the-built-in-randomplayoutestimator)) or a rescaled static evaluation, and swap in
   something stronger later; then tune `max_iterations` to your time budget, and reach for
   `initial_value_weight`/`estimate_on_expansion` once you have a cheap estimator worth trusting.
