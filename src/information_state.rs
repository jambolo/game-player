use std::collections::{HashMap, HashSet};
use crate::{Player, Color, State};

/// Tracks what information is known, unknown, and inferred
#[derive(Debug, Clone)]
pub struct InformationState {
    /// Tiles definitely known to be in opponent's hand
    known_opponent_tiles: HashSet<Tile>,
    /// Tiles that could possibly be in opponent's hand
    possible_opponent_tiles: HashSet<Tile>,
    /// Probability distribution over opponent's possible hands
    hand_probabilities: HashMap<Vec<Tile>, f64>,
    /// History of opponent moves for pattern analysis
    opponent_move_history: Vec<Move>,
    /// Inferred constraints from opponent's passes/plays
    constraints: Vec<HandConstraint>,
}

#[derive(Debug, Clone)]
pub enum HandConstraint {
    /// Opponent doesn't have any tile that can play on given number
    CannotPlay(u8),
    /// Opponent likely has a tile with given number (based on play patterns)
    LikelyHas(u8, f64), // number, probability
    /// Opponent passed when they could have played (strategic pass)
    StrategicPass { available_plays: Vec<Move> },
}