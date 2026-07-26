//! Game State Module
//!
//! This module implements the state components and traits, providing the necessary interface for the game-specific state and logic.

/// IDs of the players in a two-player game.
///
/// This enumeration defines the two possible players. The numeric values (0 and 1) can be used for array indexing and other
/// performance-critical operations. These are provided for convenience and are not required to be used.
///
/// # Examples
///
/// ```rust
/// # use game_player::PlayerId;
/// let current_player = PlayerId::Alice;
/// let player_index = current_player as usize; // 0
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerId {
    Alice = 0,
    Bob = 1,
}

impl PlayerId {
    /// Returns the other player
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use game_player::PlayerId;
    /// assert_eq!(PlayerId::Alice.other(), PlayerId::Bob);
    /// assert_eq!(PlayerId::Bob.other(), PlayerId::Alice);
    /// ```
    pub fn other(self) -> Self {
        match self {
            PlayerId::Alice => PlayerId::Bob,
            PlayerId::Bob => PlayerId::Alice,
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
/// [`minimax::search`](crate::minimax::search) relies on `fingerprint()` to key its transposition table.
/// [`mcts::search`](crate::mcts::search) does not use it at all (MCTS has no transposition table and tracks each
/// state as a distinct tree node), but the method must still be implemented since it is part of this shared trait -
/// an MCTS-only consumer may implement it trivially.
///
/// ## Turn Management
/// The trait tracks which player should move next, enabling:
/// - Alternating play enforcement
/// - Player-specific evaluation functions
/// - Turn-based game logic
///
/// Both searches rely on `whose_turn()` for adversarial correctness, but differently: `minimax::search` alternates
/// maximizing and minimizing by comparing `whose_turn()` to `PlayerId::Alice`, while `mcts::search` compares
/// `whose_turn()` for equality between a node and its parent to decide whose perspective a value is stored from.
/// Neither assumes strict ply-by-ply alternation, so games where a player moves twice in a row are supported.
///
/// ## State Transitions
/// [`apply`](State::apply) is the only way a state changes: it consumes an action and returns a new state, leaving
/// the original unchanged. Both `minimax::search` and `mcts::search` call `apply` internally to build out the states
/// they explore.
///
/// # Examples
///
/// ```rust
/// # use game_player::{State, PlayerId};
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
/// impl State for MyGameState {
///    type Action = MyAction;
///     fn fingerprint(&self) -> u64 {
///         // Generate unique hash for this position
///         // Implementation depends on game specifics
///         42 // placeholder
///     }
///
///     fn whose_turn(&self) -> PlayerId {
///         self.current_player
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
pub trait State: Clone {
    /// The type representing actions/moves in this game
    type Action: Clone;

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
    /// # use game_player::{State, PlayerId};
    /// # #[derive(Clone, Default)]
    /// # struct MyAction;
    /// # #[derive(Clone, Copy)]
    /// # struct MyGameState { current_player: PlayerId }
    /// # impl State for MyGameState {
    /// #     type Action = MyAction;
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> PlayerId { self.current_player }
    /// #     fn is_terminal(&self) -> bool { false }
    /// #     fn apply(&self, _action: &Self::Action) -> Self { *self }
    /// # }
    /// # fn create_initial_state() -> MyGameState { MyGameState { current_player: PlayerId::Alice } }
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
    /// # use game_player::{State, PlayerId};
    /// # #[derive(Clone, Default)]
    /// # struct MyAction;
    /// # #[derive(Clone, Copy)]
    /// # struct MyGameState { current_player: PlayerId }
    /// # impl State for MyGameState {
    /// #     type Action = MyAction;
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> PlayerId { self.current_player }
    /// #     fn is_terminal(&self) -> bool { false }
    /// #     fn apply(&self, _action: &Self::Action) -> Self { *self }
    /// # }
    /// let state = MyGameState { current_player: PlayerId::Alice };
    /// match state.whose_turn() {
    ///     PlayerId::Alice => println!("Alice to move"),
    ///     PlayerId::Bob => println!("Bob to move"),
    /// }
    /// ```
    fn whose_turn(&self) -> PlayerId;

    /// Checks if the game cannot continue.
    ///
    /// # Returns
    /// `true` if the game cannot continue, `false` otherwise.
    ///
    /// # Policy: Must Agree With Response Generation
    /// Neither [`minimax::search`](crate::minimax::search) nor [`mcts::search`](crate::mcts::search) calls this method - they
    /// determine that a state ends the game purely from the corresponding `ResponseGenerator::generate` returning no actions
    /// (see the policy documented on [`minimax::ResponseGenerator::generate`](crate::minimax::ResponseGenerator::generate) and
    /// [`mcts::ResponseGenerator::generate`](crate::mcts::ResponseGenerator::generate)). For the two searches to behave
    /// correctly, `is_terminal()` must therefore agree exactly with `generate`: `true` if and only if `generate` returns no
    /// actions for this state. `is_terminal()` still matters beyond the searches themselves - callers' own game loops and
    /// helpers such as the `mcts_random_playout`-gated `RandomPlayoutEstimator` rely on it directly - so it must be
    /// implemented even though the core search traversal never queries it.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::{State, PlayerId};
    /// # #[derive(Clone, Default)]
    /// # struct MyAction;
    /// # #[derive(Clone, Copy)]
    /// # struct MyGameState { game_is_over: bool }
    /// # impl State for MyGameState {
    /// #     type Action = MyAction;
    /// #     fn fingerprint(&self) -> u64 { 42 }
    /// #     fn whose_turn(&self) -> PlayerId { PlayerId::Alice }
    /// #     fn is_terminal(&self) -> bool { self.game_is_over }
    /// #     fn apply(&self, _action: &Self::Action) -> Self { *self }
    /// # }
    /// let state = MyGameState { game_is_over: true };
    /// assert!(state.is_terminal());
    /// ```
    fn is_terminal(&self) -> bool;

    /// Applies an action to the current state, returning a new state as a result of the action.
    ///
    /// The original state remains unchanged.
    ///
    /// # Arguments
    /// * `action` - The action to apply to the current state
    ///
    /// # Returns
    /// A new state representing the position after applying the action
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::{State, PlayerId};
    /// #
    /// # #[derive(Debug, Clone, Default)]
    /// # struct MyAction { move_type: String }
    /// #
    /// # #[derive(Clone)]
    /// # struct MyGameState {
    /// #     current_player: PlayerId,
    /// #     move_count: u32,
    /// #     game_over: bool
    /// # }
    /// #
    /// # impl State for MyGameState {
    /// #     type Action = MyAction;
    /// #     fn fingerprint(&self) -> u64 {
    /// #         (self.current_player as u64) << 32 | self.move_count as u64
    /// #     }
    /// #     fn whose_turn(&self) -> PlayerId { self.current_player }
    /// #     fn is_terminal(&self) -> bool { self.game_over }
    /// #     fn apply(&self, action: &Self::Action) -> Self {
    /// #         MyGameState {
    /// #             current_player: self.current_player.other(),
    /// #             move_count: self.move_count + 1,
    /// #             game_over: self.move_count >= 10,
    /// #         }
    /// #     }
    /// # }
    ///
    /// let initial_state = MyGameState {
    ///     current_player: PlayerId::Alice,
    ///     move_count: 0,
    ///     game_over: false
    /// };
    /// let action = MyAction { move_type: "play_tile".to_string() };
    ///
    /// let new_state = initial_state.apply(&action);
    ///
    /// // State should be updated
    /// assert_eq!(new_state.whose_turn(), PlayerId::Bob);
    /// assert_ne!(new_state.fingerprint(), initial_state.fingerprint());
    ///
    /// // Original state unchanged
    /// assert_eq!(initial_state.whose_turn(), PlayerId::Alice);
    /// ```
    fn apply(&self, action: &Self::Action) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_id_other() {
        assert_eq!(PlayerId::Alice.other(), PlayerId::Bob);
        assert_eq!(PlayerId::Bob.other(), PlayerId::Alice);
    }

    #[test]
    fn test_player_id_values() {
        assert_eq!(PlayerId::Alice as u8, 0);
        assert_eq!(PlayerId::Bob as u8, 1);
    }
}
