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

This crate provides components for implementing AI players in two-person games. Two searches are implemented: alpha-beta minimax and Monte Carlo Tree Search (MCTS). The public modules are `mcts`, `minimax`, `state`, `static_evaluator`, and `transposition_table` ([src/lib.rs:150-154](src/lib.rs#L150-L154)).

### Core Traits

- **`State`** ([src/state.rs](src/state.rs)) - Game state interface with associated `Action` type. Requires `fingerprint()` for transposition table hashing, `whose_turn()`, `is_terminal()`, and `apply(action)`.

- **`StaticEvaluator<S>`** ([src/static_evaluator.rs](src/static_evaluator.rs)) - Position evaluation. **Invariant: must always return values from Alice's perspective**, regardless of whose turn it is. Higher = better for Alice, lower = better for Bob. Values must lie in `[bob_wins_value(), alice_wins_value()]`. The search's max/min logic ([minimax.rs:215-217](src/minimax.rs#L215)) depends on this invariant — if you violate it by returning current-player-perspective values, the search will pick wrong moves.

- **`minimax::ResponseGenerator`** ([src/minimax.rs:86](src/minimax.rs#L86)) - Generates all legal actions from a state. Signature: `fn generate(&self, state: &Self::State, depth: u32) -> Vec<<Self::State as State>::Action>`. The search calls `state.apply(&action)` on each returned action internally.

- **`mcts::ResponseGenerator`** ([src/mcts.rs:33](src/mcts.rs#L33)) - A separate trait from `minimax::ResponseGenerator`. Signature: `fn generate(&self, state: &Self::State) -> Vec<<Self::State as State>::Action>` ([src/mcts.rs:50](src/mcts.rs#L50)) — note there is no `depth` parameter.

- **`mcts::ValueEstimator`** ([src/mcts.rs:91](src/mcts.rs#L91)) - Evaluation used in the MCTS Evaluation phase. Signature: `fn estimate(&self, state: &Self::State, rg: &Self::ResponseGenerator) -> f32` ([src/mcts.rs:116](src/mcts.rs#L116)). **Invariant: must return a value in `[0.0, 1.0]` from the perspective of the CURRENT player (`state.whose_turn()`)** — unlike `StaticEvaluator`, which is always Alice-perspective regardless of whose turn it is. `0.0` = terminal loss, `1.0` = terminal win, `0.5` = draw; for terminal states the returned value must be the exact outcome. Violating this invariant makes the search pick wrong moves.

### Search Implementation

- **`minimax::search()`** ([src/minimax.rs:172](src/minimax.rs#L172)) - Main entry point. Performs alpha-beta pruned minimax with transposition table integration. Returns `Option<S::Action>` — the best action to take. Callers who need the resulting state call `s0.apply(&action)`.

- **`mcts::search()`** ([src/mcts.rs:300](src/mcts.rs#L300)) - MCTS entry point. Signature: `fn search(s0: &S, rg: &G, estimator: &E, exploration_constant: f32, initial_value_weight: f32, estimate_on_expansion: bool, max_iterations: u32) -> Option<S::Action>`. Runs Selection/Expansion/Evaluation/Back-propagation for `max_iterations`, then returns the action leading to the root's most-visited child (or `None` if the root state has no actions). Defaults: `DEFAULT_EXPLORATION_CONSTANT` = `sqrt(2)` ([src/mcts.rs:27](src/mcts.rs#L27)), `DEFAULT_INITIAL_VALUE_WEIGHT` = `0.0` ([src/mcts.rs:30](src/mcts.rs#L30)).

- **`TranspositionTable`** ([src/transposition_table.rs](src/transposition_table.rs)) - Cache for state values and quality, keyed by fingerprint. HashMap-backed with quality-based replacement.

### Quality Semantics (Important)

The search tracks a "quality" value alongside each cached evaluation. Quality is the number of plies searched *below* the node to produce that value:

- A raw static-evaluator result has quality `0` (`SEF_QUALITY` at [src/minimax.rs:33](src/minimax.rs#L33)).
- A node fully searched to `max_depth` has the highest quality.
- When looking up a TT entry, the cached value is only reused in place of recursion if its quality meets or exceeds what the current search would need ([src/minimax.rs:261](src/minimax.rs#L261)). Otherwise the cached value is used as a preliminary estimate and the search recurses anyway.

### Alpha-Beta Pruning and the TT (Important)

When a branch is cut off by alpha-beta pruning, the resulting `best_value` is only a bound, not an exact value. The search deliberately **does not** store pruned results in the transposition table ([src/minimax.rs:320-325](src/minimax.rs#L320-L325)). Do not "optimize" this by always writing to the TT — it would poison the cache with incorrect values.

### MCTS Perspective Convention and Knobs (Important)

- **Player-who-just-moved statistics** - A node's `value_sum` and `initial_value` are stored from the perspective of the player who chose the action leading into that node — i.e. `whose_turn()` of the *parent's* state ([src/mcts.rs:164-168](src/mcts.rs#L164), back-propagation logic at [src/mcts.rs:565-585](src/mcts.rs#L565)). This is why node selection is a plain argmax of UCT at every level ([src/mcts.rs:412-439](src/mcts.rs#L412)) and the search stays correct for adversarial play without alternating max/min like `minimax::search()` does. Perspective comparisons always use `whose_turn()` equality, never ply parity, since a player may move twice in a row in some games.

- **`initial_value_weight` (w)** - A node's stored initial estimate `v0` acts as `w` virtual visits in the UCT formula ([src/mcts.rs:210-254](src/mcts.rs#L210)): `n_eff = visits + w`, `Q = (value_sum + w*v0) / n_eff`, `UCT = Q + c * sqrt(ln(parent_visits) / n_eff)`. `w = 0` (the default, `DEFAULT_INITIAL_VALUE_WEIGHT`) reduces this bit-for-bit to the classic UCT formula.

- **`estimate_on_expansion`** - `false` (lazy, default) creates and estimates exactly one child per iteration ([src/mcts.rs:452-472](src/mcts.rs#L452)). `true` (eager) creates and estimates ALL remaining untried children at once when a node is expanded, one estimator call per new child, and only the chosen child's raw estimate back-propagates ([src/mcts.rs:491-524](src/mcts.rs#L491)). Trade-off: eager costs branching-factor-times more estimator calls per expansion, so it suits cheap static evaluators rather than expensive playouts; combining eager expansion with `w = 0` wastes the stored estimates, since unvisited children then fall back to `f32::INFINITY` in UCT and are picked in arbitrary first-visit order instead of by their estimate.

### Player Convention

- **Alice** (`PlayerId::Alice = 0`) - Maximizing player
- **Bob** (`PlayerId::Bob = 1`) - Minimizing player
