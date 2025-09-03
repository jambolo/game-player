# hidden-game-player

This crate provides the base components for implementing a player in a two-person hidden information games, including game tree search algorithms and supporting infrastructure.

## Overview

The hidden-game-player crate implements the basic components of a generic player for a two-person hidden information game. It provides a complete game tree search implementation using min-max strategy with alpha-beta pruning and transposition tables for optimal performance.

## Components

### Core Traits and Types

- **`GameState` trait**: Abstract representation of game states with fingerprinting
- **`StaticEvaluator` trait**: Interface for static position evaluation functions
- **`TranspositionTable`**: High-performance cache for game state values

### Game Tree Search

- **`GameTree`**: Complete implementation of min-max search with alpha-beta pruning
- Support for configurable search depth
- Transposition table integration for performance optimization
- Comprehensive analysis and debugging features

## Key Features

- **Generic Design**: Works with any two-player hidden information game
- **High Performance**: Optimized with alpha-beta pruning and transposition tables
- **Analysis Support**: Optional detailed performance analysis and debugging
- **Memory Efficient**: Optimized data structures and memory usage

## Usage

### Basic Player Implementation

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

### Game Tree Search

```rust
use hidden_game_player::{GameTree, GameState, StaticEvaluator, TranspositionTable};
use std::sync::Arc;

// Create components
let transposition_table = Arc::new(TranspositionTable::new(1000000, 100));
let static_evaluator = Arc::new(MyEvaluator::new());
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
let mut current_state = Arc::new(my_game_state);
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
    
    fn response(&self) -> Option<Arc<dyn GameState>> {
        // Return the chosen response, if any
        todo!()
    }
    
    fn set_response(&mut self, response: Option<Arc<dyn GameState>>) {
        // Set the chosen response
        todo!()
    }
}
```

## Features

### Analysis Features
Enable detailed performance analysis:

```toml
[dependencies]
hidden-game-player = { path = "...", features = ["analysis_game_tree", "analysis_transposition_table"] }
```

### Debug Features
Enable debugging output:

```toml
[dependencies]
hidden-game-player = { path = "...", features = ["debug_game_tree_node_info"] }
```

## Dependencies

- **`serde_json`**: JSON serialization for analysis data (optional)
- **`static_assertions`**: Compile-time assertions for data structure sizes

## Performance Considerations

- Transposition table size should be tuned based on available memory
- Search depth significantly affects performance and strength
- Static evaluator quality is crucial for good play
- Analysis features add overhead and should be disabled in production

## Future Development
