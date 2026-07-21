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
//! 3. **Implement [`ResponseGenerator`](minimax::ResponseGenerator) trait**: Generates all possible moves from a position
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
//!         // Simplified: generate a few dummy moves
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
//! The crate also provides a Monte Carlo Tree Search implementation in the [`mcts`] module. Integration follows the
//! same pattern as minimax: implement [`State`], implement [`ResponseGenerator`](mcts::ResponseGenerator) (a trait
//! distinct from [`ResponseGenerator`](minimax::ResponseGenerator) in the `minimax` module - its `generate` method
//! takes no depth parameter), and implement [`ValueEstimator`](mcts::ValueEstimator), then call
//! [`search`](mcts::search).
//!
//! [`ValueEstimator`](mcts::ValueEstimator) supplies the evaluation MCTS uses in place of, or in addition to, random
//! playouts: its `estimate` method returns a value in `[0.0, 1.0]` from the perspective of the state's current
//! player (`state.whose_turn()`), where 0.0 is a loss, 1.0 is a win, and 0.5 is a draw; for terminal states the
//! returned value must be the exact outcome. Any strategy qualifies, including a random rollout to a terminal
//! state, a static evaluation function, or a neural network.
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
pub mod state;
pub mod static_evaluator;
pub mod transposition_table;

pub use state::{PlayerId, State};
pub use static_evaluator::StaticEvaluator;
