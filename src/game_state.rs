//! Generic Hidden-Information Game Player
//!
//! This module implements the components for a generic two-player hidden-information game player. It provides the necessary
//! interface for interacting with the game-specific state and logic.

/// IDs of the players in a two-player game.
///
/// This enumeration defines the two possible players. The numeric values (0 and 1) can be used for array indexing and other
/// performance-critical operations. These are provided for convenience and are not required to be used.
///
/// # Examples
///
/// ```rust
/// # use hidden_game_player::PlayerId;
/// let current_player = PlayerId::ALICE;
/// let player_index = current_player as usize; // 0
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerId {
    ALICE = 0,
    BOB = 1,
}

impl PlayerId {
    /// Returns the other player
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hidden_game_player::PlayerId;
    /// assert_eq!(PlayerId::ALICE.other(), PlayerId::BOB);
    /// assert_eq!(PlayerId::BOB.other(), PlayerId::ALICE);
    /// ```
    pub fn other(self) -> Self {
        match self {
            PlayerId::ALICE => PlayerId::BOB,
            PlayerId::BOB => PlayerId::ALICE,
        }
    }
}

/// An abstract game state that a player can interact with to analyze the game.
///
/// This trait defines the core interface that a game-specific game state must implement.
///
/// # Core Concepts
/// ## Fingerprinting
/// A game state must provide a unique fingerprint (hash) that can be used for:
/// - Transposition tables in game tree search
/// - Duplicate position detection
/// - State caching and memoization
///
/// ## Turn Management
/// The trait tracks which player should move next, enabling:
/// - Alternating play enforcement
/// - Player-specific evaluation functions
/// - Turn-based game logic
///
/// ## State Transitions
/// Game states can store references to expected responses, allowing:
/// - Pre-computed move sequences
/// - Principal variation storage
/// - Game tree navigation
///
/// # Thread Safety
/// Implementations should be thread-safe when used with `Arc` since game states may be shared between multiple analysis threads.
///
/// # Examples
///
/// ```rust
/// # use hidden_game_player::{GameState, PlayerId};
/// # use std::sync::Arc;
///
/// struct MyGameState {
///     board: [u8; 64],
///     current_player: PlayerId,
///     // other game-specific fields...
/// }
///
/// impl GameState for MyGameState {
///     fn fingerprint(&self) -> u64 {
///         // Generate unique hash for this position
///         // Implementation depends on game specifics
///         42 // placeholder
///     }
///
///     fn whose_turn(&self) -> u8 {
///         self.current_player as u8
///     }
/// }
/// ```
pub trait GameState: Sized {
    /// Returns a unique fingerprint (hash) for this game state.
    ///
    /// The fingerprint must be statistically unique across all possible game states to avoid hash collisions in transposition
    /// tables and state caches. Identical game positions must always produce identical fingerprints.
    ///
    /// # Implementation Requirements
    /// - **Deterministic**: Same position always produces same fingerprint
    /// - **Collision-resistant**: Different positions should produce different and uncorrelated fingerprints
    /// - **Fast**: Called frequently during game tree search
    /// - **Position-dependent**: Only depends on the current game state and independent of move history.
    ///
    /// # Returns
    /// A 64-bit unsigned integer representing the unique fingerprint
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hidden_game_player::{GameState, PlayerId};
    /// # struct MyGameState { current_player: PlayerId }
    /// # impl GameState for MyGameState {
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> u8 { self.current_player as u8 }
    /// # }
    /// # fn create_initial_state() -> MyGameState { MyGameState { current_player: PlayerId::ALICE } }
    /// let state = create_initial_state();
    /// let fingerprint = state.fingerprint();
    /// 
    /// // Same position should produce same fingerprint
    /// let same_state = create_initial_state();
    /// assert_eq!(fingerprint, same_state.fingerprint());
    /// ```
    fn fingerprint(&self) -> u64;

    /// Returns the ID of the player whose turn it is to move.
    ///
    /// # Returns
    /// The id of the player of the player who should move next
    ///
    /// # Examples
    /// ```rust
    /// # use hidden_game_player::{GameState, PlayerId};
    /// # struct MyGameState { current_player: PlayerId }
    /// # impl GameState for MyGameState {
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> u8 { self.current_player as u8 }
    /// # }
    /// # fn create_initial_state() -> MyGameState { MyGameState { current_player: PlayerId::ALICE } }
    /// let state = create_initial_state();
    /// match state.whose_turn() {
    ///     0 => println!("Alice to move"), // PlayerId::ALICE as u8
    ///     1 => println!("Bob to move"),   // PlayerId::BOB as u8
    ///     _ => unreachable!(),
    /// }
    /// ```
    fn whose_turn(&self) -> u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_id_other() {
        assert_eq!(PlayerId::ALICE.other(), PlayerId::BOB);
        assert_eq!(PlayerId::BOB.other(), PlayerId::ALICE);
    }

    #[test]
    fn test_player_id_values() {
        assert_eq!(PlayerId::ALICE as u8, 0);
        assert_eq!(PlayerId::BOB as u8, 1);
    }
}