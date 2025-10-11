//! Static Evaluation Function Module
//!
//! This module defines the `StaticEvaluator` trait, which provides an interface for static evaluation functions.

/// An interface for static evaluation functions.
///
/// A static evaluation function assigns a value to a game state without any lookahead. The value represents the
/// "goodness" of the state from Alice's perspective. Alice seeks to maximize the value and Bob seeks to minimize it.
///
/// The values returned by the static evaluation function should be in the range [bobWinsValue(), aliceWinsValue()].
/// If the game is over and Alice has won, then the function should return aliceWinsValue(). If the game is over and
/// Bob has won, then the function should return bobWinsValue().
///
/// # Examples
///
/// ```
/// use hidden_game_player::StaticEvaluator;
///
/// // Simple game state for demonstration
/// #[derive(Debug)]
/// struct MockGameState {
///     player_score: i32,
///     opponent_score: i32,
///     game_over: bool,
///     winner: Option<bool>, // true = Alice wins, false = Bob wins
/// }
///
/// // Simple static evaluator implementation
/// struct SimpleEvaluator;
///
/// impl StaticEvaluator<MockGameState> for SimpleEvaluator {
///     fn evaluate(&self, state: &MockGameState) -> f32 {
///         if state.game_over {
///             match state.winner {
///                 Some(true) => self.alice_wins_value(),
///                 Some(false) => self.bob_wins_value(),
///                 None => 0.0, // Draw
///             }
///         } else {
///             // Simple score difference evaluation
///             let diff = state.player_score - state.opponent_score;
///             (diff as f32) * 0.1 // Scale to reasonable range
///         }
///     }
///
///     fn alice_wins_value(&self) -> f32 {
///         1000.0
///     }
///
///     fn bob_wins_value(&self) -> f32 {
///         -1000.0
///     }
/// }
///
/// let evaluator = SimpleEvaluator;
///
/// // Test ongoing game evaluation
/// let ongoing_state = MockGameState {
///     player_score: 15,
///     opponent_score: 10,
///     game_over: false,
///     winner: None,
/// };
/// let score = evaluator.evaluate(&ongoing_state);
/// assert_eq!(score, 0.5); // (15 - 10) * 0.1
///
/// // Test Alice wins
/// let alice_wins_state = MockGameState {
///     player_score: 21,
///     opponent_score: 15,
///     game_over: true,
///     winner: Some(true),
/// };
/// assert_eq!(evaluator.evaluate(&alice_wins_state), 1000.0);
///
/// // Test Bob wins
/// let bob_wins_state = MockGameState {
///     player_score: 10,
///     opponent_score: 21,
///     game_over: true,
///     winner: Some(false),
/// };
/// assert_eq!(evaluator.evaluate(&bob_wins_state), -1000.0);
///
/// // Verify win values
/// assert_eq!(evaluator.alice_wins_value(), 1000.0);
/// assert_eq!(evaluator.bob_wins_value(), -1000.0);
/// assert!(evaluator.alice_wins_value() > evaluator.bob_wins_value());
/// ```
pub trait StaticEvaluator<G> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Mock game state for testing
    #[derive(Debug, Clone)]
    struct MockGameState {
        value: f32,
        terminal: bool,
        alice_wins: bool,
    }

    // Mock static evaluator for testing
    struct MockEvaluator;

    impl StaticEvaluator<MockGameState> for MockEvaluator {
        fn evaluate(&self, state: &MockGameState) -> f32 {
            if state.terminal {
                if state.alice_wins {
                    self.alice_wins_value()
                } else {
                    self.bob_wins_value()
                }
            } else {
                state.value
            }
        }

        fn alice_wins_value(&self) -> f32 {
            100.0
        }

        fn bob_wins_value(&self) -> f32 {
            -100.0
        }
    }

    #[test]
    fn test_static_evaluator_trait() {
        let evaluator = MockEvaluator;

        // Test ongoing game evaluation
        let ongoing_state = MockGameState {
            value: 42.5,
            terminal: false,
            alice_wins: false,
        };
        assert_eq!(evaluator.evaluate(&ongoing_state), 42.5);

        // Test Alice wins
        let alice_wins_state = MockGameState {
            value: 0.0,
            terminal: true,
            alice_wins: true,
        };
        assert_eq!(evaluator.evaluate(&alice_wins_state), 100.0);

        // Test Bob wins
        let bob_wins_state = MockGameState {
            value: 0.0,
            terminal: true,
            alice_wins: false,
        };
        assert_eq!(evaluator.evaluate(&bob_wins_state), -100.0);
    }

    #[test]
    fn test_win_values() {
        let evaluator = MockEvaluator;

        assert_eq!(evaluator.alice_wins_value(), 100.0);
        assert_eq!(evaluator.bob_wins_value(), -100.0);
        assert!(evaluator.alice_wins_value() > evaluator.bob_wins_value());
    }

    #[test]
    fn test_evaluation_consistency() {
        let evaluator = MockEvaluator;
        let state = MockGameState {
            value: 25.0,
            terminal: false,
            alice_wins: false,
        };

        // Multiple evaluations should be consistent
        let result1 = evaluator.evaluate(&state);
        let result2 = evaluator.evaluate(&state);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_evaluation_range() {
        let evaluator = MockEvaluator;

        let alice_wins_state = MockGameState {
            value: 0.0,
            terminal: true,
            alice_wins: true,
        };

        let bob_wins_state = MockGameState {
            value: 0.0,
            terminal: true,
            alice_wins: false,
        };

        let alice_score = evaluator.evaluate(&alice_wins_state);
        let bob_score = evaluator.evaluate(&bob_wins_state);

        // Alice wins should be better than Bob wins from Alice's perspective
        assert!(alice_score > bob_score);

        // Values should be within expected bounds
        assert_eq!(alice_score, evaluator.alice_wins_value());
        assert_eq!(bob_score, evaluator.bob_wins_value());
    }
}
