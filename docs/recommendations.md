# Recommendations

> **Status audit — 2026-07-20**, reviewed against current `develop` (post-refactor commits 3d397c6 and 0d5154e).
> Legend: `[x]` = **Done** · `[ ]` with a **Partial** note = partially done · `[ ]` with a ~~struck through~~ = no longer relevant · plain `[ ]` = still open.
> Line numbers in the original item text refer to pre-refactor code; current references are given in the notes.
>
> **Reorganized 2026-07-22**: the flat category list below has been split into three scopes — items that apply to the crate
> as a whole (`Library-Wide`), items specific to `minimax`, and items specific to `mcts`. Existing `[x]`/`[ ]`/status notes
> are carried over unchanged; only the grouping changed. This pass also adds a substantial new set of `mcts`-focused
> recommendations (transposition table, `ValueEstimator` simplification, and a concrete roadmap for the hidden-information
> (ISMCTS) work that motivated the earlier removal of `information_set_mcts.rs`, `belief_state.rs`, `information_state.rs`,
> `opponent_model.rs`, and `decision_engine.rs` in commit 79981b1).

---

## Library-Wide Recommendations

Applies to `state.rs`, `transposition_table.rs`, crate docs, and anything shared by both `minimax` and `mcts`.

### Code Efficiency & Idioms

* Critical:
  * [x] state.rs:31 whose_turn() returns u8 - should return PlayerId — **Done:** returns `PlayerId` (state.rs:173).

### Interface Improvements

* [x] Add TranspositionTable::stats() returning hit rate, fill factor, collision count — **Done:** `stats()` returns exactly these, gated behind the `analysis_transposition_table` feature (transposition_table.rs:251).
* [ ] Consider StateExt trait with default is_alice_turn() / is_bob_turn() helpers — still open; lower value now that `whose_turn()` returns `PlayerId` directly.
* [ ] Add a shared `move_count_hint()` convention for response generators (pre-allocation optimization) — still open. Applies independently to both `minimax::ResponseGenerator::generate()` and `mcts::ResponseGenerator::generate()`, which are separate traits with separate signatures (CLAUDE.md's "Core Traits" section) — the hint would need to be added to each, not shared via a common supertrait, since the two `generate()` signatures differ (`depth` parameter vs. none).

### Documentation Improvements

* [x] README.md examples reference removed API (response() method, GameState vs State) — **Done:** README rewritten for the current API (`State` with `apply`, `search()` returning `Option<S::Action>`).
* [ ] Add algorithm explanation section (minimax, alpha-beta, transposition tables) — **Partial:** `minimax::search()` rustdoc has a brief "Algorithm Details" bullet list (minimax.rs:165-171); no real explanation section in the README or module docs.
* [ ] Document quality semantics in transposition table (higher = deeper search) — **Partial:** documented in CLAUDE.md and in private comments (`Entry.q`, `SEF_QUALITY`), but the public rustdoc for `TranspositionTable` never defines what "quality" means. Worth fixing regardless of the MCTS-TT recommendation below, since `TranspositionTable` is a shared, general-purpose type.
* [x] ~~Document age() semantics - when to call, relationship to game turns~~ — **No longer relevant:** `age()` no longer exists in the API (removed in the refactor).
* [ ] Add performance characteristics (time/space complexity) — still open, for both searches.
* [ ] Add integration guide showing complete game implementation — **Partial:** lib.rs crate docs walk through all four integration points for both `minimax` and `mcts`, and README has a complete minimal example, but both use toy skeletons; no real-game walkthrough (e.g. tic-tac-toe) or `examples/` directory. Note: CLAUDE.md's git log references a tic-tac-toe MCTS example and a users-guide.md as already implemented in commit 79981b1 — verify whether an `examples/` directory or users-guide.md exists on disk and is just undocumented here, since the audit trail suggests it may already be partially done.

### Testing Improvements

* [x] Add transposition table collision tests — **Done:** `test_hash_collision_handling`, `test_no_eviction_from_collisions`, and `test_stats_collision_count` (transposition_table_tests.rs).
* [ ] Test NaN/Infinity handling in evaluator — **Partial:** `test_infinity_values` covers infinities in the TT, and both searches' `total_cmp()` usage makes NaN ordering deterministic, but there are no tests feeding NaN through a `StaticEvaluator` or `ValueEstimator`.
* [ ] Add property-based tests with proptest for `State` invariants — still open (no proptest dependency). Would benefit both `minimax` and `mcts`, since both depend on the same `State` contract (fingerprint stability, `apply` purity).

---

## Minimax Recommendations

Applies to `minimax.rs` only.

### Minimax Code Efficiency & Idioms

* High:
  * [x] minimax.rs:95-310 and minimax.rs:312-427: alice_search/bob_search are 200+ lines of duplication. Unify into single parameterized function with is_maximizing: bool or generic over player — **Done:** unified into `search_recursive()` (minimax.rs:192) with `maximizing` derived from the `player` parameter.
  * [x] minimax.rs:59 ResponseGenerator::generate() returns Vec\<Box\<Self::State\>\> - consider returning impl Iterator or using stack allocation for small move counts — **Done (superseded):** commit 0d5154e changed `generate()` to return `Vec<Action>`; the boxed-state allocation is gone. Returning `impl Iterator` or a SmallVec remains an optional micro-optimization.
* Medium:
  * [x] minimax.rs uses context.tt.borrow_mut() repeatedly - cache the borrow across related operations — **Done:** `get_preliminary_value()` holds one borrow across the check + update (minimax.rs:374-381); the only other call site borrows once (minimax.rs:323).
  * [x] Float comparisons without NaN guards - use total_cmp() (stabilized in 1.62) — **Done:** all float comparisons in the search use `total_cmp()` (minimax.rs:216, 221, 238-240, 285-300).
  * [x] minimax.rs:23 PhantomData\<S\> unnecessary - S already constrained via other fields — **Done:** `Context` (minimax.rs:51) has no `PhantomData`; `S` is constrained through `sef`/`rg`.
* Low:
  * [ ] Move ordering sorts entire list - use select_nth_unstable_by() for partial sort when only top moves needed — still open: all candidates are fully sorted at every node (minimax.rs:237-241). **Downgraded to low priority (2026-07-20):** state generation and evaluation in `generate_candidates()` almost certainly dominate the sort cost, so the gain is likely minimal. Note also that no fixed partial-sort size is safe — how many candidates the loop consumes depends on runtime alpha/beta cutoffs — so an implementation would need a lazy scheme (deferred tail sort or heap-based selection) rather than a fixed `k`.

### Minimax Interface Improvements

* [ ] Add builder pattern for minimax::search() configuration (depth, time limit, node limit) — still open; the only configuration is `max_depth`.
* [ ] Return Result\<Response\<S\>, SearchError\> instead of Option\<Response\<S\>\> for better error context — still open. Note: the signature is now `Option<S::Action>` (`Response` is internal), so the target would be `Result<S::Action, SearchError>`.

### Features to Add

None of these are implemented yet (verified 2026-07-20 — no matching code in `src/`).

* High value:
  * [ ] Iterative deepening with aspiration windows — blocked by the current API: `search()` constructs a fresh internal `TranspositionTable` per call (minimax.rs:179), so the TT cannot be reused across iterations.
  * [ ] Principal variation (PV) extraction and storage
  * [ ] Killer move heuristic (store refutation moves per ply)
  * [ ] History heuristic for move ordering
  * [ ] Null-move pruning (optional, for applicable games)
  * [ ] Quiescence search hook (for games with captures/checks)
* Medium value:
  * [ ] Time management (search with time budget, not just depth)
  * [ ] Multi-PV search for analysis
  * [ ] Pondering support (search during opponent's turn)
  * [ ] Zobrist hashing utilities for implementing fingerprint() — shared value, since both `minimax` and (per the ISMCTS recommendations below) a future information-set-keyed `mcts` would benefit, but listed here as it is currently only consumed by `minimax`.
* Analysis:
  * [ ] Node count statistics — TT-level counters (checks/hits/collisions) exist behind `analysis_transposition_table`, but search node visits are not tracked.
  * [ ] Branching factor tracking
  * [ ] Pruning effectiveness metrics

### Minimax Documentation Improvements

Stale-doc leftovers spotted during the 2026-07-20 audit:

* [x] `search()` rustdoc lists a `tt` argument that no longer exists (minimax.rs:137), and the module doc claims the TT "can be reused across multiple searches" (minimax.rs:24) though it is now created internally per call. — **Done:** removed the `tt` entry and the stale `A` type parameter from `search()`'s rustdoc (the doc now matches `search<S, E, R>`), and the module doc now says a TT is created internally for each search.
* [x] The lib.rs doc example's evaluator multiplies by `if state.current_player { 1.0 } else { -1.0 }` (lib.rs:89) — it flips sign by turn, violating the documented always-from-Alice's-perspective invariant. — **Done:** the example evaluator now returns `state.board.count_ones() as f32 - 16.0` unconditionally (Alice's perspective).

### Minimax Testing Improvements

* [ ] Add alpha-beta pruning verification (instrument node visits, compare to plain minimax) — **Partial:** `test_alpha_beta_pruning_correctness` verifies the pruned search still returns the correct move, but there is no node-visit instrumentation or comparison against plain minimax.
* [ ] Add large branching factor stress test — still open (`test_large_table` exercises TT size only, not search branching).
* [ ] Test iterative deepening TT reuse (when implemented) — blocked: iterative deepening is not implemented.
* [ ] Benchmark suite comparing with/without TT, different table sizes — still open (no `benches/` directory).

---

## MCTS Recommendations

Applies to `mcts.rs`. This section is organized around the task's two explicit asks — adding a transposition table and
simplifying/enhancing `ValueEstimator` — followed by a broader set of established-technique gaps, and closing with a
concrete roadmap toward the hidden-information (ISMCTS) work that is this crate's stated end goal for the module.

### Transposition Table for MCTS

`mcts::search()` currently builds a plain tree with no TT (mcts.rs:52 — "states reached by different move orders are
treated as distinct nodes").

* [ ] **[Low risk]** Value-caching TT: on node creation, check `state.fingerprint()` against a `TranspositionTable`
  and seed `initial_value` from a hit instead of spending an estimator call; write converged values back. Reuses
  `TranspositionTable` as-is — mirrors minimax's `get_preliminary_value()` (minimax.rs:367). Recommended starting point.
* [ ] **[High risk]** True transposition-aware search (tree → DAG, merging nodes for the same state reached via
  different move orders). Doesn't fit the current single-parent-per-node model (`Node::uct()`/`back_propagate()`
  derive perspective from a node's one parent, mcts.rs:284/659) — would need per-edge, not per-node, stats. A
  structural rewrite, not an incremental add.
* [ ] Tree reuse across successive `search()` calls (re-root the existing `Arena` at the actually-played child
  instead of rebuilding from scratch each move). A related but distinct feature from the TT itself.

### `ValueEstimator` Simplification & Enhancement

* [ ] Ship a built-in `StaticEvaluator` → `ValueEstimator` adapter. The perspective-flip/normalization formula is
  currently hand-duplicated in four places (mcts.rs:163-167, static_evaluator.rs:14-16, CLAUDE.md, lib.rs:241-247).
* [x] Ship a built-in random-playout estimator (module docs mention rollout as a strategy, mcts.rs:6-7, but none
  ships). Would need `rand` as a new, feature-gated dependency. — **Done:** `random_playout::RandomPlayoutEstimator<G>`,
  gated behind the new `mcts_random_playout` feature (`dep:rand`, default-off) so consumers supplying their own
  estimator aren't forced to pull in `rand`. Samples a uniformly-random action from `rg.generate(state)` and applies
  it until terminal. Since a generic `State` has no way to read off "who won," this also introduces a new
  `TerminalOutcome: State` trait (`fn outcome(&self) -> f32`, same `[0.0, 1.0]`/current-player-perspective contract as
  `ValueEstimator`) that `G::State` must implement for the estimator to be usable. The estimator owns a seeded
  `StdRng` (`RandomPlayoutEstimator::new(seed)`) behind a `RefCell` so playouts stay reproducible despite
  `estimate(&self, ...)`'s `&self` signature.
* [ ] Add a batched `estimate_batch()` hook (default: maps `estimate()`) so `expand_eager`'s per-child loop
  (mcts.rs:596-610) can be overridden for NN-style vectorized inference.
* [ ] Memoize `estimate()` via the transposition table above — same mechanism as the TT item, but framed as the
  answer to "how often does `estimate()` need to run" rather than a new trait shape.

### Other Established-Technique Gaps

* [ ] Document/fix the tie-break for the root's best-child choice — `max_by` on visits (mcts.rs:472-476) silently
  picks the *last* maximal child, an order-dependent tie-break driven by child-insertion order.
* [ ] Wall-clock/time-budgeted search, alongside `max_iterations` (mcts.rs:401) — MCTS is naturally anytime.
* [ ] RAVE / AMAF (share action-value stats across sibling subtrees for faster early convergence).
* [ ] Root parallelization (run independent `search()`s concurrently, sum root child visits) as the low-risk entry
  point to parallelism; full tree/leaf parallelization (virtual loss, shared arena) is a larger, separate effort.
* [ ] Switch `Node::state` from `S` to `Rc<S>` (mcts.rs:209) for consistency with minimax's `Rc<S>` sharing
  (minimax.rs:38) and to cut copy/memory cost, especially before tree-reuse or TT work keeps nodes alive longer.
* [ ] Analysis/introspection parity with minimax's TT `stats()` — node count, max depth, root child visit
  distribution, as a first real consumer of the unused `analysis_game_tree`/`debug_game_tree_node_info` flags.

### Hidden Information Games (ISMCTS) Roadmap

Stated end goal for `mcts`, and the reason `information_set_mcts.rs`, `belief_state.rs`, `information_state.rs`,
`opponent_model.rs`, and `decision_engine.rs` were deleted in commit 79981b1. Having read the deleted versions (`git
show f1328d4:...`): **they should not be reinstated as-is** — they're hard-coded to a domino-like game and predate
the current generic trait architecture entirely. Recommend a from-scratch design instead:

* [ ] New `InformationSet` trait, kept distinct from `State` (a player's partial view isn't a full state, and
  overloading `State::fingerprint()`'s "position-dependent, move-history-independent" contract would break existing
  `minimax`/`mcts` consumers).
* [ ] New `Determinizer` trait: `determinize(&self, info_set, rng: &mut impl Rng) -> State`. The one place real
  randomness enters the crate — thread an explicit RNG rather than storing one, to keep searches reproducible.
* [ ] A new `mcts::ismcts` module with its own `Node`/`select`/`expand`/`back_propagate`, not a reuse of
  `mcts::Node` — ISMCTS nodes need per-action *availability* counts (not just visits) since different
  determinizations can make different actions legal, which the current fixed `untried_actions` list can't express.
  Should still call the existing, unmodified `mcts::ResponseGenerator`/`ValueEstimator` traits against each
  iteration's determinized concrete `State`.
* [ ] Key/fingerprint nodes by information-set identity, not `State::fingerprint()`.
* [ ] Generalize `BeliefState`/`OpponentModel` into an optional, pluggable component a `Determinizer` can consult for
  non-uniform sampling — not mandatory, since uniform-random determinization is enough for many hidden-information
  games.
* [ ] Don't reinstate a generic `decision_engine.rs` — it was application-level "player" glue, not library surface;
  build the equivalent wiring in a new example instead.
* [ ] Gate the new module behind a Cargo feature (e.g. `hidden_information`) while it stabilizes, consistent with
  the existing `analysis_*` flag pattern, so it doesn't destabilize `mcts`'s existing test suite.
* [ ] Add a concrete hidden-information example/test game (Kuhn poker is the standard minimal case) under
  `examples/`/`tests/`, both to validate convergence and to document how to implement `InformationSet`/`Determinizer`.

### MCTS Testing Improvements

* [ ] Add call-count tests for `expand_eager`'s batching once `estimate_batch()` exists (cf. `test_eager_estimator_call_count`, mcts.rs:1161).
* [ ] Add a with/without-TT benchmark on a game with real transpositions once the value-caching TT exists (current fixtures are transposition-free trees).
* [ ] Add a Kuhn-poker-or-equivalent convergence test once the ISMCTS module exists.

---

## Prioritized Quick Wins

* [x] ~~Fix edition = "2024" → "2021"~~ — **No longer relevant:** the project deliberately targets edition 2024 (see CLAUDE.md), and the code uses let-chains (transposition_table.rs:128), which require it.
* [x] whose_turn() -\> PlayerId instead of u8 — **Done.** *(Library-wide)*
* [x] Unify alice_search/bob_search — **Done.** *(Minimax)*
* [x] Add TranspositionTable::stats() — **Done** (feature-gated). *(Library-wide)*
* [x] Update README examples — **Done.** *(Library-wide)*
* [ ] Add the value-caching MCTS transposition table (see "Transposition Table for MCTS" above) — the single highest
  leverage, lowest-risk new item from this pass; it reuses `TranspositionTable` unchanged. *(MCTS)*
* [ ] Ship the `StaticEvaluatorEstimator` adapter (see "`ValueEstimator` Simplification & Enhancement" above) — small,
  mechanical, and eliminates a four-times-duplicated formula. *(MCTS)*
