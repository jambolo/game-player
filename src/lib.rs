//! Game Player
//!
//! This crate provides the foundational traits and structures needed to implement a player for two-person,
//! perfect-information games, offering both a minimax search and a Monte Carlo Tree Search.
//!
//! # Minimax Search
//!
//! The crate includes a minimax search implementation with alpha-beta pruning and a transposition table to optimize performance.
//!
//! ## Key Integration Points
//!
//! 1. **Implement [`State`] trait**: Provides game state management and move application with associated Action type
//! 2. **Implement [`StaticEvaluator`] trait**: Evaluates how good a position is for each player
//! 3. **Implement [`ResponseGenerator`](minimax::ResponseGenerator) trait**: Generates all possible moves from a
//!    position - returning no moves is the search's sole signal that the position ends the game, so a non-terminal
//!    position must always yield at least one move (e.g. an explicit "pass"); see the policy documented on
//!    [`ResponseGenerator::generate`](minimax::ResponseGenerator::generate)
//! 4. **Use [`search`](minimax::search)**: Combines everything to find the optimal move
//!
//! ## Example
//!
//! ```rust
//! use game_player::{PlayerId, State, StaticEvaluator};
//! use game_player::minimax::{ResponseGenerator, search};
//!
//! // Simple game structures (chess-like for demonstration)
//! #[derive(Debug, Clone, PartialEq)]
//! struct GameMove { from: (u8, u8), to: (u8, u8) }
//!
//! #[derive(Debug, Clone)]
//! struct GameState {
//!     board: u64,               // Simplified board representation
//!     current_player: PlayerId, // Alice = white, Bob = black
//!     move_count: u32,
//! }
//!
//! impl GameState {
//!     fn new() -> Self {
//!         Self { board: 0x1234567890abcdef, current_player: PlayerId::Alice, move_count: 0 }
//!     }
//!
//!     fn is_game_over(&self) -> bool { self.move_count > 50 }
//!
//!     fn get_possible_moves(&self) -> Vec<GameMove> {
//!         // Simplified: generate a few dummy moves. A real implementation must return at least one
//!         // move whenever `is_game_over()` is false (e.g. a "pass" move if the rules force one) and no
//!         // moves when `is_game_over()` is true - see the ResponseGenerator::generate policy docs.
//!         vec![
//!             GameMove { from: (0, 0), to: (1, 1) },
//!             GameMove { from: (0, 1), to: (1, 0) },
//!             GameMove { from: (1, 0), to: (2, 0) },
//!         ]
//!     }
//! }
//!
//! // 1. Implement the State trait for your game
//! impl State for GameState {
//!     type Action = GameMove;
//!
//!     fn fingerprint(&self) -> u64 {
//!         // Create unique hash for transposition table
//!         self.board ^ (self.current_player as u64) << 63 ^ self.move_count as u64
//!     }
//!
//!     fn whose_turn(&self) -> PlayerId {
//!         self.current_player
//!     }
//!
//!     fn is_terminal(&self) -> bool {
//!         self.is_game_over()
//!     }
//!
//!     fn apply(&self, game_move: &Self::Action) -> Self {
//!         // Apply move and return new state
//!         Self {
//!             board: self.board.wrapping_add(1), // Simplified board update
//!             current_player: self.current_player.other(),
//!             move_count: self.move_count + 1,
//!         }
//!     }
//! }
//!
//! // 2. Implement static evaluation for your game
//! struct GameEvaluator;
//!
//! impl StaticEvaluator for GameEvaluator {
//!     type State = GameState;
//!
//!     fn evaluate(&self, state: &GameState) -> f32 {
//!         if state.is_game_over() {
//!             return 0.0; // Draw
//!         }
//!         // Always from Alice's perspective: material advantage relative to baseline
//!         state.board.count_ones() as f32 - 16.0
//!     }
//!
//!     fn alice_wins_value(&self) -> f32 { 1000.0 }
//!     fn bob_wins_value(&self) -> f32 { -1000.0 }
//! }
//!
//! // 3. Implement move generation for your game
//! struct GameMoveGenerator;
//!
//! impl ResponseGenerator for GameMoveGenerator {
//!     type State = GameState;
//!
//!     fn generate(&self, state: &Self::State, _depth: u32) -> Vec<GameMove> {
//!         state.get_possible_moves()
//!     }
//! }
//!
//! // 4. Use the minimax search to find the best move
//! fn find_best_move() -> Option<GameMove> {
//!     // Set up the game components
//!     let initial_state = GameState::new();
//!     let evaluator = GameEvaluator;
//!     let move_generator = GameMoveGenerator;
//!
//!     // Perform minimax search to find best action
//!     search(&evaluator, &move_generator, &initial_state, 6)
//! }
//!
//! // Usage: Create an AI that can play your game
//! let best_move = find_best_move();
//! match best_move {
//!     Some(action) => println!("AI found best move: {:?}", action),
//!     None => println!("No moves available"),
//! }
//! ```
//!
//! # Monte Carlo Tree Search
//!
//! The crate also provides a Monte Carlo Tree Search (MCTS) implementation in the [`mcts`] module. Integration follows
//! the same shape as minimax, but the pieces play a different role.
//!
//! ## Key Integration Points
//!
//! 1. **Implement [`State`] trait**: The same trait used by minimax - game state management and move application
//! 2. **Implement [`ValueEstimator`](mcts::ValueEstimator) trait**: Evaluates a state in `[0.0, 1.0]` from the
//!    perspective of the current player (`state.whose_turn()`), in place of - or in addition to - random playouts
//! 3. **Implement [`ResponseGenerator`](mcts::ResponseGenerator) trait**: Generates all possible moves from a
//!    position - a trait distinct from [`ResponseGenerator`](minimax::ResponseGenerator) in the `minimax` module,
//!    since its `generate` method takes no `depth` parameter. The same no-legal-moves policy applies: see
//!    [`ResponseGenerator::generate`](mcts::ResponseGenerator::generate)
//! 4. **Use [`search`](mcts::search)**: Combines everything to find the most-visited move
//!
//! ## Example
//!
//! ```rust
//! use game_player::{PlayerId, State, StaticEvaluator};
//! use game_player::mcts::{ResponseGenerator, ValueEstimator, search};
//!
//! // Same game structures as the minimax example above
//! #[derive(Debug, Clone, PartialEq)]
//! struct GameMove { from: (u8, u8), to: (u8, u8) }
//!
//! #[derive(Debug, Clone)]
//! struct GameState {
//!     board: u64,               // Simplified board representation
//!     current_player: PlayerId, // Alice = white, Bob = black
//!     move_count: u32,
//! }
//!
//! impl GameState {
//!     fn new() -> Self {
//!         Self { board: 0x1234567890abcdef, current_player: PlayerId::Alice, move_count: 0 }
//!     }
//!
//!     fn is_game_over(&self) -> bool { self.move_count > 50 }
//!
//!     fn get_possible_moves(&self) -> Vec<GameMove> {
//!         // Simplified: generate a few dummy moves. A real implementation must return at least one
//!         // move whenever `is_game_over()` is false (e.g. a "pass" move if the rules force one) and no
//!         // moves when `is_game_over()` is true - see the ResponseGenerator::generate policy docs.
//!         vec![
//!             GameMove { from: (0, 0), to: (1, 1) },
//!             GameMove { from: (0, 1), to: (1, 0) },
//!             GameMove { from: (1, 0), to: (2, 0) },
//!         ]
//!     }
//! }
//!
//! // 1. Implement the State trait for your game (identical to the minimax example)
//! impl State for GameState {
//!     type Action = GameMove;
//!
//!     fn fingerprint(&self) -> u64 {
//!         // MCTS does not use a transposition table, so this can be trivial for
//!         // an MCTS-only consumer. It is still required because it is part of
//!         // the shared State trait.
//!         self.board ^ (self.current_player as u64) << 63 ^ self.move_count as u64
//!     }
//!
//!     fn whose_turn(&self) -> PlayerId {
//!         self.current_player
//!     }
//!
//!     fn is_terminal(&self) -> bool {
//!         self.is_game_over()
//!     }
//!
//!     fn apply(&self, game_move: &Self::Action) -> Self {
//!         Self {
//!             board: self.board.wrapping_add(1), // Simplified board update
//!             current_player: self.current_player.other(),
//!             move_count: self.move_count + 1,
//!         }
//!     }
//! }
//!
//! // 2. Implement mcts::ResponseGenerator - note there is no depth parameter, unlike
//! // minimax::ResponseGenerator
//! struct GameMoveGenerator;
//!
//! impl ResponseGenerator for GameMoveGenerator {
//!     type State = GameState;
//!
//!     fn generate(&self, state: &Self::State) -> Vec<GameMove> {
//!         state.get_possible_moves()
//!     }
//! }
//!
//! // 3. Implement ValueEstimator. Here we adapt an existing Alice-perspective
//! // StaticEvaluator: normalize its output into [0.0, 1.0], then flip perspective for
//! // Bob so the result is always from state.whose_turn()'s perspective.
//! struct GameEvaluator;
//!
//! impl StaticEvaluator for GameEvaluator {
//!     type State = GameState;
//!
//!     fn evaluate(&self, state: &GameState) -> f32 {
//!         if state.is_game_over() {
//!             return 0.0; // Draw
//!         }
//!         // Always from Alice's perspective: material advantage relative to baseline
//!         state.board.count_ones() as f32 - 16.0
//!     }
//!
//!     fn alice_wins_value(&self) -> f32 { 1000.0 }
//!     fn bob_wins_value(&self) -> f32 { -1000.0 }
//! }
//!
//! struct GameValueEstimator { sef: GameEvaluator }
//!
//! impl ValueEstimator for GameValueEstimator {
//!     type State = GameState;
//!     type ResponseGenerator = GameMoveGenerator;
//!
//!     fn estimate(&self, state: &GameState, _rg: &GameMoveGenerator) -> f32 {
//!         let eval = self.sef.evaluate(state);
//!         let v01 = (eval - self.sef.bob_wins_value())
//!             / (self.sef.alice_wins_value() - self.sef.bob_wins_value());
//!         if state.whose_turn() == PlayerId::Alice { v01 } else { 1.0 - v01 }
//!     }
//! }
//!
//! // 4. Use the MCTS search to find the best move
//! fn find_best_move_mcts() -> Option<GameMove> {
//!     // Set up the game components
//!     let initial_state = GameState::new();
//!     let move_generator = GameMoveGenerator;
//!     let estimator = GameValueEstimator { sef: GameEvaluator };
//!
//!     // Perform MCTS search to find the most-visited action
//!     search(
//!         &initial_state,
//!         &move_generator,
//!         &estimator,
//!         game_player::mcts::DEFAULT_EXPLORATION_CONSTANT,
//!         game_player::mcts::DEFAULT_INITIAL_VALUE_WEIGHT,
//!         false, // lazy expansion
//!         1000,  // iterations
//!     )
//! }
//!
//! // Usage: Create an AI that can play your game
//! let best_move = find_best_move_mcts();
//! match best_move {
//!     Some(action) => println!("MCTS found best move: {:?}", action),
//!     None => println!("No moves available"),
//! }
//! ```
//!
//! ## Node Perspective Convention
//!
//! A node's statistics (`value_sum`, `initial_value`) are stored from the perspective of the player who chose the
//! action leading into it - i.e. the `whose_turn()` of the *parent's* state. This is why Selection is a plain argmax
//! of UCT at every level of the tree (no alternating max/min as in `minimax::search`), and it stays correct for
//! adversarial play even in games where a player may move twice in a row, since perspective comparisons always use
//! `whose_turn()` equality rather than ply parity.
//!
//! ## Tuning Knobs
//!
//! Two parameters tune the search. `initial_value_weight` blends a node's initial value estimate into the UCT
//! formula as a number of virtual visits; a weight of `0.0` disables the blend, reducing to the standard UCT
//! formula. `estimate_on_expansion` selects between lazy expansion (`false`, the default: one child is created and
//! estimated per iteration) and eager expansion (`true`: every untried child of a node is created and estimated at
//! once) - eager expansion trades more estimator calls per expansion for fewer iterations, so it suits cheap
//! estimators such as a static evaluation function rather than expensive playouts.
//! [`DEFAULT_EXPLORATION_CONSTANT`](mcts::DEFAULT_EXPLORATION_CONSTANT) and
//! [`DEFAULT_INITIAL_VALUE_WEIGHT`](mcts::DEFAULT_INITIAL_VALUE_WEIGHT) provide reasonable starting values for the
//! exploration constant and initial-value weight, respectively.

pub mod mcts;
pub mod minimax;
#[cfg(feature = "mcts_random_playout")]
pub mod random_playout;
pub mod state;
pub mod static_evaluator;
pub mod transposition_table;

pub use state::{PlayerId, State};
pub use static_evaluator::StaticEvaluator;
