# game-player

This crate provides the base components for implementing a player in a two-person game.

**WORK IN PROGRESS**

![Rust Workflow](https://github.com/jambolo/game-player/actions/workflows/rust.yml/badge.svg)
[![codecov](https://codecov.io/gh/jambolo/game-player/branch/master/graph/badge.svg)](https://codecov.io/gh/jambolo/game-player)

## Overview

The game-player crate implements the basic components of a generic player in a two-person game. It provides:
1. A min-max game tree search algorithm using alpha-beta pruning and transposition tables for optimal performance.
2. A basic Monte Carlo Tree Search algorithm.

## Components

### Common Core Traits and Types

- **`State` trait**: Abstract representation of game states with fingerprinting
- **`PlayerId`**: Two players, Alice and Bob, 0 and 1
- **`StaticEvaluator` trait**: Interface for static position evaluation functions
- **`TranspositionTable`**: Cache for game state values

### Minimax Search

- Complete implementation of min-max search with alpha-beta pruning
- **`ResponseGenerator` trait**: Trait that generates all possible responses to a state
- Support for configurable search depth
- Transposition table integration with relevance and value quality enhancements.
- Supports two-player game only

### Monte Carlo Tree Search

- **`MonteCarloTreeSearch`**: Monte Carlo Tree Search implementation with UCT-based node selection.
- **`ResponseGenerator` trait**: Trait that generates all possible actions from a state
- Configurable iteration count
- Configurable exploration constant
- Supports two-player game only

## Usage

### Basic MCTS Player Implementation

```rust
use hidden_game_player::{Player, GameState, Action};

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

### Basic Minimax Player Implementation

```rust
use hidden_game_player::{Action, GameTree, GameState, StaticEvaluator, TranspositionTable};
use std::sync::Rc;

// Create components
let transposition_table = Rc::new(TranspositionTable::new(1000000, 100));
let static_evaluator = Rc::new(MyEvaluator::new());
let response_generator = Box::new(|state, depth| {
    // Generate all possible moves from this state
    generate_moves(state, depth)
});

// Create game tree
let game_tree = GameTree::new(
    transposition_table,
    static_evaluator,
    response_generator,
    8  // search depth
);

// Find best move
let mut current_state = Rc::new(my_game_state);
game_tree.find_best_response(&mut current_state);
```

### Custom Game State

```rust
use hidden_game_player::GameState;

struct MyGameState {
    // Your game state data
}

impl GameState for MyGameState {
    fn fingerprint(&self) -> u64 {
        // Return unique fingerprint for this state
        todo!()
    }

    fn whose_turn(&self) -> Self::PlayerId {
        // Return which player moves next
        todo!()
    }

    fn response(&self) -> Option<Rc<dyn GameState>> {
        // Return the chosen response, if any
        todo!()
    }

    fn set_response(&mut self, response: Option<Rc<dyn GameState>>) {
        // Set the chosen response
        todo!()
    }
}
```

## Features

## Future Development
