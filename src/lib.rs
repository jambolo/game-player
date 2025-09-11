//! Game Player
//!
//! This crate provides the foundational traits and structures needed to implement a player for two-person perfect and hidden
//! information games.

pub mod game_state;
pub mod game_tree;
pub mod static_evaluator;
pub mod transposition_table;

pub use game_state::{GameState, PlayerId};
pub use game_tree::{GameTree, ResponseGenerator};
pub use static_evaluator::StaticEvaluator;
pub use transposition_table::TranspositionTable;
