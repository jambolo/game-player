use crate::game_state::GameState;

/// An interface for static evaluation functions.
///
/// A static evaluation function assigns a value to a game state without any lookahead. The value represents the
/// "goodness" of the state from Alice's perspective. Alice seeks to maximize the value and Bob seeks to minimize it.
///
/// The values returned by the static evaluation function should be in the range [bobWinsValue(), aliceWinsValue()].
/// If the game is over and Alice has won, then the function should return aliceWinsValue(). If the game is over and
/// Bob has won, then the function should return bobWinsValue().
pub trait StaticEvaluator<G: GameState> {
    /// Evaluates the given state and returns its value from Alice's perspective.
    ///
    /// # Arguments
    /// * `state` - The state to be evaluated
    ///
    /// # Returns
    /// The value of the state from Alice's perspective
    ///
    /// # Note
    /// This function must be implemented.
    fn evaluate(&self, state: &G) -> f32;

    /// Returns the value that indicates that Alice has won.
    ///
    /// # Returns
    /// The value that indicates that Alice has won
    ///
    /// # Note
    /// This function must be implemented.
    fn alice_wins_value(&self) -> f32;

    /// Returns the value that indicates that Bob has won.
    ///
    /// # Returns
    /// The value that indicates that Bob has won
    ///
    /// # Note
    /// This function must be implemented.
    fn bob_wins_value(&self) -> f32;
}
