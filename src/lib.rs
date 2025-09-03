//! A library providing the basic components of a generic player for a two-person hidden information game.
//! 
//! This crate implements game tree search using min-max strategy, alpha-beta pruning, and transposition tables.

pub mod game_state;
pub mod game_tree;
pub mod static_evaluator;
pub mod transposition_table;

pub use game_state::{GameState, PlayerId};
pub use game_tree::GameTree;
pub use static_evaluator::StaticEvaluator;
pub use transposition_table::TranspositionTable;

