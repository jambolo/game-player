# game-player

This crate provides the base components for implementing a player in a two-person game.

This is a **WORK IN PROGRESS**

| Branch    | Workflow Status | Coverage |
|-----------|-----------------|----------|
| `master`  | ![Release](https://github.com/jambolo/game-player/actions/workflows/release.yml/badge.svg?branch=master) | N/A |
| `develop` | ![Rust](https://github.com/jambolo/game-player/actions/workflows/rust.yml/badge.svg?branch=develop) | [![codecov](https://codecov.io/gh/jambolo/game-player/branch/develop/graph/badge.svg)](https://codecov.io/gh/jambolo/game-player) |

## Overview

The game-player crate provides the core traits and search components needed to build a player for a two-person game. The current
minimax entry point is `game_player::minimax::search(&evaluator, &response_generator, &state, max_depth)`, which returns the best
resulting state as `Option<Rc<S>>`.

It provides:

1. A min-max game tree search algorithm using alpha-beta pruning and transposition tables for optimal performance.
2. A basic Monte Carlo Tree Search algorithm.

## Components

### Common Core Traits and Types

- **`State` trait**: Abstract representation of game states with fingerprinting, turn tracking, terminal detection, and state
  transitions via `apply`
- **`PlayerId`**: Two players, Alice and Bob
- **`StaticEvaluator` trait**: Interface for evaluating a position from Alice's perspective. Has an associated `State` type.

### Minimax Search

- Complete implementation of min-max search with alpha-beta pruning and transposition-table-backed move ordering
- **`ResponseGenerator` trait**: Trait that generates all possible resulting states from a position.
- **`search` function**: `search(&evaluator, &response_generator, &state, max_depth) -> Option<S>`
- Support for configurable search depth
- Internal transposition table integration for cached evaluations.
- Supports two-player game only

### Monte Carlo Tree Search

- **`MonteCarloTreeSearch`**: Monte Carlo Tree Search implementation with UCT-based node selection.
- **`ResponseGenerator` trait**: Trait that generates all possible actions from a state
- Configurable iteration count
- Configurable exploration constant
- Supports two-player game only

## Usage

### Basic Minimax Player Implementation

```rust
use game_player::{PlayerId, State, StaticEvaluator};
use game_player::minimax::{search, ResponseGenerator};

#[derive(Clone)]
struct MyAction;

#[derive(Clone)]
struct MyGameState {
    current_player: PlayerId,
    moves_remaining: u8,
}

impl State for MyGameState {
    type Action = MyAction;

    fn fingerprint(&self) -> u64 {
        ((self.current_player as u64) << 8) | self.moves_remaining as u64
    }

    fn whose_turn(&self) -> PlayerId {
        self.current_player
    }

    fn is_terminal(&self) -> bool {
        self.moves_remaining == 0
    }

    fn apply(&self, _action: &Self::Action) -> Self {
        Self {
            current_player: self.current_player.other(),
            moves_remaining: self.moves_remaining.saturating_sub(1),
        }
    }
}

struct MyEvaluator;

impl StaticEvaluator for MyEvaluator {
    type State = MyGameState;

    fn evaluate(&self, state: &MyGameState) -> f32 {
        if state.is_terminal() {
            0.0
        } else if state.whose_turn() == PlayerId::Alice {
            1.0
        } else {
            -1.0
        }
    }

    fn alice_wins_value(&self) -> f32 {
        1000.0
    }

    fn bob_wins_value(&self) -> f32 {
        -1000.0
    }
}

struct MyResponseGenerator;

impl ResponseGenerator for MyResponseGenerator {
    type State = MyGameState;

    fn generate(&self, state: &Self::State, _depth: i32) -> Vec<Self::State> {
        if state.is_terminal() {
            Vec::new()
        } else {
            vec![state.apply(&MyAction)]
        }
    }
}

let initial_state = MyGameState {
    current_player: PlayerId::Alice,
    moves_remaining: 4,
};

let evaluator = MyEvaluator;
let response_generator = MyResponseGenerator;

let best_state = search(&evaluator, &response_generator, &initial_state, 6);

match best_state {
    Some(state) => println!("Best resulting state has {} moves remaining", state.moves_remaining),
    None => println!("No legal responses available"),
}
```

### Basic MCTS Player Implementation

```rust
use game_player::{Player, GameState, Action};

struct MyPlayer {
    name: String,
}

impl Player for MyPlayer {
    fn setup(&mut self, game_state: &mut DominoesGameState) {
        // Initialize player's hand from the boneyard
    }

    fn my_turn(&mut self, game_state: &DominoesGameState) -> (Action, DominoesGameState) {
        // Implement your turn logic here
        todo!("Implement turn logic")
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

## Features

## Future Development
