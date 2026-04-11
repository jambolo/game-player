# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                    # Build the crate
cargo test                     # Run all tests with default features enabled
cargo test <test_name>         # Run a single test
cargo test --features analysis_game_tree  # Test with analysis features enabled
cargo clippy                   # Lint
cargo fmt                      # Format code
```

The crate uses `edition = "2024"` ([Cargo.toml](Cargo.toml)), which requires a recent stable rustc. Avoid suggesting edition-2021-only syntax.

## Feature Flags

These flags are declared but are currently placeholders for future analysis/serialization output — no `#[cfg(feature = ...)]` code paths exist yet.

- `analysis_game_tree` - Will enable game tree analysis
- `analysis_game_state` - Will enable game state analysis
- `analysis_transposition_table` - Enables transposition table analysis
- `debug_game_tree_node_info` - Will enable debug info for game tree nodes

## Architecture

This crate provides components for implementing AI players in two-person games. The only search currently implemented is alpha-beta minimax; the [README.md](README.md) mentions a Monte Carlo Tree Search, but MCTS is **not yet implemented** — it is planned future work. The public modules are `minimax`, `state`, `static_evaluator`, and `transposition_table` ([src/lib.rs:129-132](src/lib.rs#L129-L132)).

### Core Traits

- **`State`** ([src/state.rs](src/state.rs)) - Game state interface with associated `Action` type. Requires `fingerprint()` for transposition table hashing, `whose_turn()`, `is_terminal()`, and `apply(action)`.

- **`StaticEvaluator<S>`** ([src/static_evaluator.rs](src/static_evaluator.rs)) - Position evaluation. **Always from Alice's perspective**, regardless of whose turn it is: higher = better for Alice, lower = better for Bob. Values must lie in `[bob_wins_value(), alice_wins_value()]`. This invariant is load-bearing for the search's max/min logic — do not "flip the sign for Bob's turn" in an evaluator.

- **`ResponseGenerator`** ([src/minimax.rs:88](src/minimax.rs#L88)) - Generates all legal moves from a state. Signature: `fn generate(&self, state: &Self::State, depth: u32) -> Vec<Self::State>`. Returns plain owned states, not boxed or `Rc`-wrapped.

### Search Implementation

- **`minimax::search()`** ([src/minimax.rs:167](src/minimax.rs#L167)) - Main entry point. Performs alpha-beta pruned minimax with transposition table integration. Returns `Option<S>` with the best resulting state (the internal `Rc` is unwrapped before returning).

- **`TranspositionTable`** ([src/transposition_table.rs](src/transposition_table.rs)) - Cache for state values and quality, keyed by fingerprint. HashMap-backed with quality-based replacement.

### Quality Semantics (Important)

The search tracks a "quality" value alongside each cached evaluation. Quality is the number of plies searched *below* the node to produce that value:

- A raw static-evaluator result has quality `0` (`SEF_QUALITY` at [src/minimax.rs:32](src/minimax.rs#L32)).
- A node fully searched to `max_depth` has the highest quality.
- When looking up a TT entry, the cached value is only reused in place of recursion if its quality meets or exceeds what the current search would need ([src/minimax.rs:252](src/minimax.rs#L252)). Otherwise the cached value is used as a preliminary estimate and the search recurses anyway.

### Alpha-Beta Pruning and the TT (Important)

When a branch is cut off by alpha-beta pruning, the resulting `best_value` is only a bound, not an exact value. The search deliberately **does not** store pruned results in the transposition table ([src/minimax.rs:321-326](src/minimax.rs#L321-L326)). Do not "optimize" this by always writing to the TT — it would poison the cache with incorrect values.

### Player Convention

- **Alice** (`PlayerId::Alice = 0`) - Maximizing player
- **Bob** (`PlayerId::Bob = 1`) - Minimizing player
