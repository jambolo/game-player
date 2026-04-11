# Recommendations

> **Status audit — 2026-07-20**, reviewed against current `develop` (post-refactor commits 3d397c6 and 0d5154e).
> Legend: `[x]` = **Done** · `[ ]` with a **Partial** note = partially done · `[ ]` with a ~~struck through~~ = no longer relevant · plain `[ ]` = still open.
> Line numbers in the original item text refer to pre-refactor code; current references are given in the notes.

## Code Efficiency & Idioms

* Critical:
  * [x] state.rs:31 whose_turn() returns u8 - should return PlayerId — **Done:** returns `PlayerId` (state.rs:173).
* High:
  * [x] minimax.rs:95-310 and minimax.rs:312-427: alice_search/bob_search are 200+ lines of duplication. Unify into single parameterized function with is_maximizing: bool or generic over player — **Done:** unified into `search_recursive()` (minimax.rs:192) with `maximizing` derived from the `player` parameter.
  * [x] minimax.rs:59 ResponseGenerator::generate() returns Vec\<Box\<Self::State\>\> - consider returning impl Iterator or using stack allocation for small move counts — **Done (superseded):** commit 0d5154e changed `generate()` to return `Vec<Action>`; the boxed-state allocation is gone. Returning `impl Iterator` or a SmallVec remains an optional micro-optimization.
* Medium:
  * [x] minimax.rs uses context.tt.borrow_mut() repeatedly - cache the borrow across related operations — **Done:** `get_preliminary_value()` holds one borrow across the check + update (minimax.rs:374-381); the only other call site borrows once (minimax.rs:323).
  * [x] Float comparisons without NaN guards - use total_cmp() (stabilized in 1.62) — **Done:** all float comparisons in the search use `total_cmp()` (minimax.rs:216, 221, 238-240, 285-300).
  * [x] minimax.rs:23 PhantomData\<S\> unnecessary - S already constrained via other fields — **Done:** `Context` (minimax.rs:51) has no `PhantomData`; `S` is constrained through `sef`/`rg`.
* Low:
  * [ ] Move ordering sorts entire list - use select_nth_unstable_by() for partial sort when only top moves needed — still open: all candidates are fully sorted at every node (minimax.rs:237-241). **Downgraded to low priority (2026-07-20):** state generation and evaluation in `generate_candidates()` almost certainly dominate the sort cost, so the gain is likely minimal. Note also that no fixed partial-sort size is safe — how many candidates the loop consumes depends on runtime alpha/beta cutoffs — so an implementation would need a lazy scheme (deferred tail sort or heap-based selection) rather than a fixed `k`.

## Interface Improvements

* [ ] Add builder pattern for minimax::search() configuration (depth, time limit, node limit) — still open; the only configuration is `max_depth`.
* [ ] Return Result\<Response\<S\>, SearchError\> instead of Option\<Response\<S\>\> for better error context — still open. Note: the signature is now `Option<S::Action>` (`Response` is internal), so the target would be `Result<S::Action, SearchError>`.
* [x] Add TranspositionTable::stats() returning hit rate, fill factor, collision count — **Done:** `stats()` returns exactly these, gated behind the `analysis_transposition_table` feature (transposition_table.rs:251).
* [ ] Consider StateExt trait with default is_alice_turn() / is_bob_turn() helpers — still open; lower value now that `whose_turn()` returns `PlayerId` directly.
* [ ] Add ResponseGenerator::move_count_hint() for pre-allocation optimization — still open.

## Features to Add

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
  * [ ] Zobrist hashing utilities for implementing fingerprint()
* Analysis:
  * [ ] Node count statistics — TT-level counters (checks/hits/collisions) exist behind `analysis_transposition_table`, but search node visits are not tracked.
  * [ ] Branching factor tracking
  * [ ] Pruning effectiveness metrics

## Documentation Improvements

* [x] README.md examples reference removed API (response() method, GameState vs State) — **Done:** README rewritten for the current API (`State` with `apply`, `search()` returning `Option<S::Action>`).
* [ ] Add algorithm explanation section (minimax, alpha-beta, transposition tables) — **Partial:** `search()` rustdoc has a brief "Algorithm Details" bullet list (minimax.rs:165-171); no real explanation section in the README or module docs.
* [ ] Document quality semantics in transposition table (higher = deeper search) — **Partial:** documented in CLAUDE.md and in private comments (`Entry.q`, `SEF_QUALITY`), but the public rustdoc for `TranspositionTable` never defines what "quality" means.
* [x] ~~Document age() semantics - when to call, relationship to game turns~~ — **No longer relevant:** `age()` no longer exists in the API (removed in the refactor).
* [ ] Add performance characteristics (time/space complexity) — still open.
* [ ] Add integration guide showing complete game implementation — **Partial:** lib.rs crate docs walk through all four integration points and README has a complete minimal example, but both use toy skeletons; no real-game walkthrough (e.g. tic-tac-toe) or `examples/` directory.

Stale-doc leftovers spotted during this audit (not in the original list):

* [x] `search()` rustdoc lists a `tt` argument that no longer exists (minimax.rs:137), and the module doc claims the TT "can be reused across multiple searches" (minimax.rs:24) though it is now created internally per call. — **Done:** removed the `tt` entry and the stale `A` type parameter from `search()`'s rustdoc (the doc now matches `search<S, E, R>`), and the module doc now says a TT is created internally for each search.
* [x] The lib.rs doc example's evaluator multiplies by `if state.current_player { 1.0 } else { -1.0 }` (lib.rs:89) — it flips sign by turn, violating the documented always-from-Alice's-perspective invariant. — **Done:** the example evaluator now returns `state.board.count_ones() as f32 - 16.0` unconditionally (Alice's perspective).

## Testing Improvements

* [ ] Add alpha-beta pruning verification (instrument node visits, compare to plain minimax) — **Partial:** `test_alpha_beta_pruning_correctness` verifies the pruned search still returns the correct move, but there is no node-visit instrumentation or comparison against plain minimax.
* [x] Add transposition table collision tests — **Done:** `test_hash_collision_handling`, `test_no_eviction_from_collisions`, and `test_stats_collision_count` (transposition_table_tests.rs).
* [ ] Add large branching factor stress test — still open (`test_large_table` exercises TT size only, not search branching).
* [ ] Test NaN/Infinity handling in evaluator — **Partial:** `test_infinity_values` covers infinities in the TT, and the search's `total_cmp()` makes NaN ordering deterministic, but there are no tests feeding NaN through an evaluator.
* [ ] Test iterative deepening TT reuse (when implemented) — blocked: iterative deepening is not implemented.
* [ ] Add property-based tests with proptest for State invariants — still open (no proptest dependency).
* [ ] Benchmark suite comparing with/without TT, different table sizes — still open (no `benches/` directory).

## Prioritized Quick Wins

* [x] ~~Fix edition = "2024" → "2021"~~ — **No longer relevant:** the project deliberately targets edition 2024 (see CLAUDE.md), and the code uses let-chains (transposition_table.rs:128), which require it.
* [x] whose_turn() -\> PlayerId instead of u8 — **Done.**
* [x] Unify alice_search/bob_search — **Done.**
* [x] Add TranspositionTable::stats() — **Done** (feature-gated).
* [x] Update README examples — **Done.**
