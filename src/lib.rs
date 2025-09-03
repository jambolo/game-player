//! Game Player
//!
//! This crate provides the foundational traits and structures needed to implement a player for two-person perfect and hidden
//! information games.
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
//! 5. **Use [`TranspositionTable`]**: Caches evaluations for better performance
//!
//! ## Example
//!
//! ```rust
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use game_player::{PlayerId, State, StaticEvaluator, TranspositionTable};
//! use game_player::minimax::{ResponseGenerator, search};
//!
//! // Simple game structures (chess-like for demonstration)
//! #[derive(Debug, Clone, PartialEq)]
//! struct GameMove { from: (u8, u8), to: (u8, u8) }
//!
//! #[derive(Debug, Clone)]
//! struct GameState {
//!     board: u64,           // Simplified board representation
//!     current_player: bool, // true = white/alice, false = black/bob
//!     move_count: u32,
//! }
//!
//! impl GameState {
//!     fn new() -> Self {
//!         Self { board: 0x1234567890abcdef, current_player: true, move_count: 0 }
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
//!     fn whose_turn(&self) -> u8 {
//!         if self.current_player { PlayerId::ALICE as u8 } else { PlayerId::BOB as u8 }
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
//!             current_player: !self.current_player,
//!             move_count: self.move_count + 1,
//!         }
//!     }
//! }
//!
//! // 2. Implement static evaluation for your game
//! struct GameEvaluator;
//!
//! impl StaticEvaluator<GameState> for GameEvaluator {
//!     fn evaluate(&self, state: &GameState) -> f32 {
//!         if state.is_game_over() {
//!             return 0.0; // Draw
//!         }
//!         // Simple evaluation: favor the player with more "material" (simplified)
//!         (state.board.count_ones() as f32 - 16.0) * if state.current_player { 1.0 } else { -1.0 }
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
//!     fn generate(&self, state: &Rc<Self::State>, _depth: i32) -> Vec<Box<Self::State>> {
//!         state.get_possible_moves()
//!             .into_iter()
//!             .map(|game_move| Box::new(state.apply(&game_move)))
//!             .collect()
//!     }
//! }
//!
//! // 4. Use the minimax search to find the best move
//! fn find_best_move() -> Option<GameState> {
//!     // Set up the game components
//!     let initial_state = Rc::new(GameState::new());
//!     let evaluator = GameEvaluator;
//!     let move_generator = GameMoveGenerator;
//!     let transposition_table = Rc::new(RefCell::new(TranspositionTable::new(10000, 100)));
//!
//!     // Perform minimax search to find best move
//!     search(&transposition_table, &evaluator, &move_generator, &initial_state, 6)
//!         .map(|best_state| (*best_state).clone())
//! }
//!
//! // Usage: Create an AI that can play your game
//! let best_move = find_best_move();
//! match best_move {
//!     Some(new_state) => println!("AI found best move, new state: {:?}", new_state),
//!     None => println!("No moves available"),
//! }
//! ```

pub mod mcts;
pub mod minimax;
pub mod state;
pub mod static_evaluator;
pub mod transposition_table;

pub use state::{PlayerId, State};
pub use static_evaluator::StaticEvaluator;
pub use transposition_table::TranspositionTable;
