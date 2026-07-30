# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

The crate uses `edition = "2024"` ([Cargo.toml](Cargo.toml)), which requires a recent stable rustc. Avoid suggesting edition-2021-only syntax.

## Feature Flags

The feature flags declared in [Cargo.toml](Cargo.toml) are currently placeholders for future analysis/serialization output — no `#[cfg(feature = ...)]` code paths exist yet.

## Architecture

### Core Traits

- **`StaticEvaluator<S>`** ([src/static_evaluator.rs](src/static_evaluator.rs)) - Position evaluation. **Invariant: must always return values from Alice's perspective**, regardless of whose turn it is. Higher = better for Alice, lower = better for Bob. Values must lie in `[bob_wins_value(), alice_wins_value()]`. The search's max/min logic ([minimax.rs:215-217](src/minimax.rs#L215)) depends on this invariant — if you violate it by returning current-player-perspective values, the search will pick wrong moves.

- **`minimax::ResponseGenerator`** ([src/minimax.rs:88](src/minimax.rs#L88)) - Generates all legal actions from a state. Signature: `fn generate(&self, state: &Self::State, depth: u32) -> Vec<<Self::State as State>::Action>`. The search calls `state.apply(&action)` on each returned action internally. Must return no actions if and only if the state is terminal — see "No-Legal-Moves Policy" below.

- **`mcts::ResponseGenerator`** ([src/mcts.rs:88](src/mcts.rs#L88)) - A separate trait from `minimax::ResponseGenerator`. Signature: `fn generate(&self, state: &Self::State) -> Vec<<Self::State as State>::Action>` ([src/mcts.rs:119](src/mcts.rs#L119)) — note there is no `depth` parameter. Same no-legal-moves policy as the minimax variant.

- **`mcts::ValueEstimator`** ([src/mcts.rs:163](src/mcts.rs#L163)) - Evaluation used in the MCTS Evaluation phase. Signature: `fn estimate(&self, state: &Self::State, rg: &Self::ResponseGenerator) -> f32` ([src/mcts.rs:198](src/mcts.rs#L198)). **Invariant: must return a value in `[0.0, 1.0]` from the perspective of the CURRENT player (`state.whose_turn()`)** — unlike `StaticEvaluator`, which is always Alice-perspective regardless of whose turn it is. `0.0` = terminal loss, `1.0` = terminal win, `0.5` = draw; for terminal states the returned value must be the exact outcome. Violating this invariant makes the search pick wrong moves. **`0.0`/`1.0` are reserved for certainty** (a terminal state or a rollout that reached one) — a heuristic that stops short of the end of the game must stay strictly inside `(0.0, 1.0)` even when very confident, or it becomes indistinguishable from a proven win/loss in the search's statistics.

### No-Legal-Moves Policy (Important)

An empty result from `ResponseGenerator::generate` (both the `minimax` and `mcts` variants) is the crate's sole and definitive signal that a state ends the game. Neither `minimax::search` nor `mcts::search` calls `State::is_terminal()` — so a `generate` implementation must return **no actions if and only if `state.is_terminal()` is `true`**:

- If `is_terminal()` is `false`, `generate` must return at least one action. Add an explicit "pass" action for a state that forces a player to skip a turn without ending the game (`apply`-ing it typically just flips `whose_turn()`).
- If `is_terminal()` is `true`, `generate` must return no actions. A forced resignation ends the game immediately, so model it by making `is_terminal()` `true` for that state rather than returning a "resign" action.

Both searches validate this at every node with a `debug_assert_eq!` immediately after calling `generate` (`generate_candidates` in [minimax.rs:365](src/minimax.rs#L365), `Node::new` in [mcts.rs:261](src/mcts.rs#L261)), so a violation panics immediately in a debug or test build instead of silently truncating the search one ply early and trusting a stale static value/estimate. Because of this invariant, `mcts::selectable()` ([src/mcts.rs:514](src/mcts.rs#L514)) doesn't need to check `state.is_terminal()` itself — `!has_children` alone already identifies a terminal node, since `Node::new` populates `untried_actions` from the same `generate()` call and nothing but `expand`/`expand_eager` ever removes from it.

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
