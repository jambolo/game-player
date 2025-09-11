//! Game State Interface
//!
//! This module implements the state components and traits, providing the necessary interface for the game-specific state and
//! logic.

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

/// A trait representing the state.
///
/// This trait defines the core interface that a game-specific state must implement.
///
/// # Core Concepts
/// ## Fingerprinting
/// A state must provide a unique fingerprint (hash) that can be used for:
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
/// # Examples
///
/// ```rust
/// # use hidden_game_player::{State, PlayerId};
///
/// #[derive(Clone, Default)]
/// struct MyAction;
///
/// #[derive(Clone, Copy)]
/// struct MyGameState {
///     board: [u8; 64],
///     current_player: PlayerId,
///     game_over: bool,
///     // other game-specific fields...
/// }
///
/// impl State<MyAction> for MyGameState {
///     fn fingerprint(&self) -> u64 {
///         // Generate unique hash for this position
///         // Implementation depends on game specifics
///         42 // placeholder
///     }
///
///     fn whose_turn(&self) -> u8 {
///         self.current_player as u8
///     }
///
///     fn is_terminal(&self) -> bool {
///         self.game_over
///     }
///
///     fn apply(&self, _action: &MyAction) -> Self {
///         MyGameState {
///             board: self.board,
///             current_player: self.current_player.other(),
///             game_over: false,
///         }
///     }
/// }
/// ```
pub trait State<A>: Sized {
    /// Returns a unique fingerprint (hash) for this state.
    ///
    /// The fingerprint must be statistically unique across all possible game states to avoid hash collisions in transposition
    /// tables and state caches. Identical game positions must always produce identical fingerprints.
    ///
    /// # Implementation Requirements
    /// - **Deterministic**: Same position always produces same fingerprint
    /// - **Collision-resistant**: Different positions should produce different and uncorrelated fingerprints
    /// - **Fast**: Called frequently during game tree search
    /// - **Position-dependent**: Only depends on the current state and independent of move history.
    ///
    /// # Returns
    /// A 64-bit unsigned integer representing the unique fingerprint
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hidden_game_player::{State, PlayerId};
    /// # #[derive(Clone, Default)]
    /// # struct MyAction;
    /// # #[derive(Clone, Copy)]
    /// # struct MyGameState { current_player: PlayerId }
    /// # impl State<MyAction> for MyGameState {
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> u8 { self.current_player as u8 }
    /// #     fn is_terminal(&self) -> bool { false }
    /// #     fn apply(&self, _action: &MyAction) -> Self { *self }
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
    /// # use hidden_game_player::{State, PlayerId};
    /// # #[derive(Clone, Default)]
    /// # struct MyAction;
    /// # #[derive(Clone, Copy)]
    /// # struct MyGameState { current_player: PlayerId }
    /// # impl State<MyAction> for MyGameState {
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> u8 { self.current_player as u8 }
    /// #     fn is_terminal(&self) -> bool { false }
    /// #     fn apply(&self, _action: &MyAction) -> Self { *self }
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

    /// Checks if the game cannot continue.
    ///
    /// # Returns
    /// `true` if the game cannot continue, `false` otherwise.
    ///
    /// # Examples
    /// ```rust
    /// # use hidden_game_player::{State, PlayerId};
    /// # #[derive(Clone, Default)]
    /// # struct MyAction;
    /// # #[derive(Clone, Copy)]
    /// # struct MyGameState { game_is_over: bool }
    /// # impl State<MyAction> for MyGameState {
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> u8 { 0 }
    /// #     fn is_terminal(&self) -> bool { self.game_is_over }
    /// #     fn apply(&self, _action: &MyAction) -> Self { *self }
    /// # }
    /// let state = MyGameState { game_is_over: true };
    /// assert!(state.is_terminal());
    /// ```
    fn is_terminal(&self) -> bool;

    /// Applies an action to the current state, returning a new state as a result of the action.
    ///
    /// This method creates a new state by applying the given action to the current state.
    /// The original state remains unchanged (immutable transformation).
    ///
    /// # Arguments
    /// * `action` - The action to apply to the current state
    ///
    /// # Returns
    /// A new state representing the position after applying the action
    ///
    /// # Examples
    /// ```rust
    /// # use hidden_game_player::{State, PlayerId};
    /// #
    /// # #[derive(Debug, Clone, Default)]
    /// # struct MyAction { move_type: String }
    /// #
    /// # struct MyGameState {
    /// #     current_player: PlayerId,
    /// #     move_count: u32,
    /// #     game_over: bool
    /// # }
    /// #
    /// # impl State<MyAction> for MyGameState {
    /// #     fn fingerprint(&self) -> u64 {
    /// #         (self.current_player as u64) << 32 | self.move_count as u64
    /// #     }
    /// #     fn whose_turn(&self) -> u8 { self.current_player as u8 }
    /// #     fn is_terminal(&self) -> bool { self.game_over }
    /// #     fn apply(&self, action: &MyAction) -> Self {
    /// #         MyGameState {
    /// #             current_player: self.current_player.other(),
    /// #             move_count: self.move_count + 1,
    /// #             game_over: self.move_count >= 10,
    /// #         }
    /// #     }
    /// # }
    ///
    /// let initial_state = MyGameState {
    ///     current_player: PlayerId::ALICE,
    ///     move_count: 0,
    ///     game_over: false
    /// };
    /// let action = MyAction { move_type: "play_tile".to_string() };
    ///
    /// let new_state = initial_state.apply(&action);
    ///
    /// // State should be updated
    /// assert_eq!(new_state.whose_turn(), PlayerId::BOB as u8);
    /// assert_ne!(new_state.fingerprint(), initial_state.fingerprint());
    ///
    /// // Original state unchanged
    /// assert_eq!(initial_state.whose_turn(), PlayerId::ALICE as u8);
    /// ```
    fn apply(&self, action: &A) -> Self;
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
