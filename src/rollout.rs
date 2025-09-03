//! This module contains the Rollout trait used by the game-specific code to implement rollouts.
//! ! The Rollout trait defines the interface for performing rollouts in a game state.
//! ! It includes methods for selecting moves, playing random moves, and evaluating the game state.
//! ! The trait is generic over the game state type, allowing it to be used with different games.
//! ! The Rollout trait is used by the MCTS algorithm to simulate game play and evaluate the potential outcomes of different moves.
//! ! The Rollout trait is implemented for specific game states in the game-specific code.
//!

use crate::state::State;
use crate::mcts::MCTSPlayer;
///