# game-player

This crate provides the base components for implementing a player in a two-person game.

This is a **WORK IN PROGRESS**

`develop` branch: ![Rust](https://github.com/jambolo/game-player/actions/workflows/rust.yml/badge.svg?branch=develop) [![codecov](https://codecov.io/gh/jambolo/game-player/branch/develop/graph/badge.svg)](https://codecov.io/gh/jambolo/game-player)

## Overview

The game-player crate provides the core traits and search components needed to build a player for a two-person game.

It provides:

1. A min-max game tree search algorithm using alpha-beta pruning and transposition tables for optimal performance.

## Components

### Common Core Traits and Types

- **`State` trait**: Abstract representation of game states with fingerprinting, turn tracking, terminal detection, and state
  transitions via `apply`
- **`PlayerId`**: Two players, Alice and Bob
- **`StaticEvaluator` trait**: Interface for evaluating a position from Alice's perspective. Has an associated `State` type.

### Minimax Search

- Complete implementation of min-max search with alpha-beta pruning and transposition-table optimizations.
- **`ResponseGenerator` trait**: Trait that generates all possible actions from a state.
- **`search` function**: `search(&evaluator, &response_generator, &state, max_depth) -> Option<S::Action>`
- Support for configurable search depth
- Internal transposition table integration for cached evaluations.
- Supports two-player game only

## Usage

### Basic Minimax Player Implementation

```rust
use game_player::{PlayerId, State, StaticEvaluator};
use game_player::minimax::{search, ResponseGenerator};

#[derive(Clone, Debug)]
struct MyAction {
    // game-specific move data
}

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

    // Always evaluated from Alice's perspective: higher is better for Alice,
    // lower is better for Bob. Never flip the sign based on whose turn it is.
    fn evaluate(&self, state: &MyGameState) -> f32 {
        state.moves_remaining as f32
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

    fn generate(&self, state: &Self::State, _depth: u32) -> Vec<MyAction> {
        if state.is_terminal() {
            Vec::new()
        } else {
            vec![MyAction { /* ... */ }]
        }
    }
}

let initial_state = MyGameState {
    current_player: PlayerId::Alice,
    moves_remaining: 4,
};

let evaluator = MyEvaluator;
let response_generator = MyResponseGenerator;

match search(&evaluator, &response_generator, &initial_state, 6) {
    Some(action) => {
        let next_state = initial_state.apply(&action);
        println!("Best resulting state has {} moves remaining", next_state.moves_remaining);
    }
    None => println!("No legal responses available"),
}
```

## Features

## Future Development

- Monte Carlo Tree Search (MCTS) with UCT-based node selection
