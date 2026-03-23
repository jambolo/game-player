# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                    # Build the crate
cargo test                     # Run all tests
cargo test <test_name>         # Run a single test
cargo test --features analysis_game_tree  # Build with analysis features
cargo clippy                   # Lint
cargo fmt                      # Format code
```

## Feature Flags

- `analysis_game_tree` - Enables JSON serialization for game tree analysis
- `analysis_game_state` - Enables JSON serialization for game state analysis
- `analysis_transposition_table` - Enables JSON serialization for transposition table analysis
- `debug_game_tree_node_info` - Enables debug info for game tree nodes

## Architecture

This crate provides components for implementing AI players in two-person games using minimax search with alpha-beta pruning.

### Core Traits (implement these for your game)

- **`State`** (`state.rs`): Game state with fingerprinting, turn tracking, and action application. Has associated `Action` type.
- **`StaticEvaluator<S>`** (`static_evaluator.rs`): Position evaluation from Alice's perspective. Returns values in range `[bob_wins_value, alice_wins_value]`.
- **`ResponseGenerator`** (`minimax.rs`): Generates all legal moves from a position. Returns empty vec when no moves available.

### Search Infrastructure

- **`minimax::search()`**: Main entry point. Takes transposition table, evaluator, response generator, state, and depth.
- **`TranspositionTable`**: Caches state values by fingerprint. Uses quality-based replacement and age-based eviction.
- **`PlayerId`**: Two players - `ALICE` (0, maximizing) and `BOB` (1, minimizing).

### Integration Pattern

1. Implement `State` for your game state type
2. Implement `StaticEvaluator` for position evaluation
3. Implement `ResponseGenerator` for move generation
4. Call `minimax::search()` with `Rc<RefCell<TranspositionTable>>`

## Branch Strategy

- `master` - Release branch (triggers version tagging)
- `develop` - Main development branch
- `feature/**` - Feature branches (CI runs on push)
