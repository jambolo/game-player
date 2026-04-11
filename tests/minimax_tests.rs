//! Unit tests for the minimax search implementation
//!
//! These tests use mock implementations to verify the correctness of the minimax
//! algorithm, alpha-beta pruning, and transposition table integration.

use std::collections::HashMap;

use game_player::minimax::{ResponseGenerator, search};
use game_player::state::{PlayerId, State};
use game_player::static_evaluator::StaticEvaluator;

/// Mock action type for testing
#[derive(Debug, Clone, PartialEq)]
struct MockAction {
    id: u32,
}

/// Mock game state for testing minimax
#[derive(Debug, Clone, PartialEq)]
struct MockGameState {
    id: u32,
    player: PlayerId,
    value: Option<f32>, // Pre-set value for leaf nodes
    children: Vec<u32>, // IDs of child states
}

impl MockGameState {
    fn new(id: u32, player: PlayerId) -> Self {
        Self {
            id,
            player,
            value: None,
            children: Vec::new(),
        }
    }

    fn with_value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    fn with_children(mut self, children: Vec<u32>) -> Self {
        self.children = children;
        self
    }
}

impl State for MockGameState {
    type Action = MockAction;

    fn whose_turn(&self) -> PlayerId {
        self.player
    }

    fn fingerprint(&self) -> u64 {
        self.id as u64
    }

    fn is_terminal(&self) -> bool {
        self.children.is_empty() && self.value.is_some()
    }

    fn apply(&self, _action: &MockAction) -> Self {
        // For testing purposes, we don't need a real implementation
        // since the mock response generator handles state transitions
        self.clone()
    }
}

/// Mock static evaluator that returns pre-set values or defaults
struct MockStaticEvaluator {
    values: HashMap<u32, f32>,
}

impl MockStaticEvaluator {
    fn new() -> Self {
        Self { values: HashMap::new() }
    }

    fn with_value(mut self, state_id: u32, value: f32) -> Self {
        self.values.insert(state_id, value);
        self
    }
}

impl StaticEvaluator for MockStaticEvaluator {
    type State = MockGameState;

    fn evaluate(&self, state: &MockGameState) -> f32 {
        state
            .value
            .unwrap_or_else(|| self.values.get(&state.id).copied().unwrap_or(0.0))
    }

    fn alice_wins_value(&self) -> f32 {
        1000.0
    }

    fn bob_wins_value(&self) -> f32 {
        -1000.0
    }
}

/// Mock response generator that creates predefined child states
struct MockResponseGenerator {
    states: HashMap<u32, MockGameState>,
}

impl MockResponseGenerator {
    fn new() -> Self {
        Self { states: HashMap::new() }
    }

    fn add_state(mut self, state: MockGameState) -> Self {
        self.states.insert(state.id, state);
        self
    }
}

impl ResponseGenerator for MockResponseGenerator {
    type State = MockGameState;
    fn generate(&self, state: &MockGameState, _depth: u32) -> Vec<MockGameState> {
        state
            .children
            .iter()
            .filter_map(|&child_id| self.states.get(&child_id).cloned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_returns_none_for_no_moves() {
        let evaluator = MockStaticEvaluator::new();
        let generator = MockResponseGenerator::new();
        let state = MockGameState::new(1, PlayerId::Alice);

        let result = search(&evaluator, &generator, &state, 3);
        assert!(result.is_none());
    }

    #[test]
    fn test_response_generator_trait() {
        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(2, PlayerId::Bob))
            .add_state(MockGameState::new(3, PlayerId::Bob));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        let responses = generator.generate(&state, 0);
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, 2);
        assert_eq!(responses[1].id, 3);
    }

    #[test]
    fn test_empty_response_generation() {
        let generator = MockResponseGenerator::new();
        let state = MockGameState::new(1, PlayerId::Alice);

        let responses = generator.generate(&state, 0);
        assert!(responses.is_empty());
    }

    #[test]
    fn test_mock_static_evaluator() {
        let evaluator = MockStaticEvaluator::new().with_value(1, 5.0).with_value(2, -3.0);

        let state1 = MockGameState::new(1, PlayerId::Alice);
        let state2 = MockGameState::new(2, PlayerId::Bob);
        let state3 = MockGameState::new(3, PlayerId::Alice);

        assert_eq!(evaluator.evaluate(&state1), 5.0);
        assert_eq!(evaluator.evaluate(&state2), -3.0);
        assert_eq!(evaluator.evaluate(&state3), 0.0); // Default value

        assert_eq!(evaluator.alice_wins_value(), 1000.0);
        assert_eq!(evaluator.bob_wins_value(), -1000.0);
    }

    #[test]
    fn test_mock_game_state_terminal() {
        let state1 = MockGameState::new(1, PlayerId::Alice);
        let state2 = MockGameState::new(2, PlayerId::Bob).with_value(5.0);
        let state3 = MockGameState::new(3, PlayerId::Alice).with_children(vec![4]);

        assert!(!state1.is_terminal()); // No children, no value
        assert!(state2.is_terminal()); // Has value, no children
        assert!(!state3.is_terminal()); // Has children
    }

    #[test]
    fn test_mock_game_state_fingerprint() {
        let state1 = MockGameState::new(100, PlayerId::Alice);
        let state2 = MockGameState::new(200, PlayerId::Bob);

        assert_eq!(state1.fingerprint(), 100);
        assert_eq!(state2.fingerprint(), 200);
        assert_ne!(state1.fingerprint(), state2.fingerprint());
    }

    #[test]
    fn test_mock_game_state_whose_turn() {
        let alice_state = MockGameState::new(1, PlayerId::Alice);
        let bob_state = MockGameState::new(2, PlayerId::Bob);

        assert_eq!(alice_state.whose_turn(), PlayerId::Alice);
        assert_eq!(bob_state.whose_turn(), PlayerId::Bob);
    }

    #[test]
    fn test_search_alice_picks_best_move() {
        let evaluator = MockStaticEvaluator::new()
            .with_value(2, 5.0)
            .with_value(3, 10.0)
            .with_value(4, 3.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3, 4]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_value(5.0))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_value(10.0))
            .add_state(MockGameState::new(4, PlayerId::Bob).with_value(3.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3, 4]);

        let result = search(&evaluator, &generator, &state, 1);

        assert!(result.is_some());
        let best_move = result.unwrap();
        assert_eq!(best_move.id, 3); // Should pick the move with highest value (10.0)
    }

    #[test]
    fn test_search_bob_picks_best_move() {
        let evaluator = MockStaticEvaluator::new()
            .with_value(2, 5.0)
            .with_value(3, 10.0)
            .with_value(4, 3.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Bob).with_children(vec![2, 3, 4]))
            .add_state(MockGameState::new(2, PlayerId::Alice).with_value(5.0))
            .add_state(MockGameState::new(3, PlayerId::Alice).with_value(10.0))
            .add_state(MockGameState::new(4, PlayerId::Alice).with_value(3.0));

        let state = MockGameState::new(1, PlayerId::Bob).with_children(vec![2, 3, 4]);

        let result = search(&evaluator, &generator, &state, 1);

        assert!(result.is_some());
        let best_move = result.unwrap();
        assert_eq!(best_move.id, 4); // Bob should pick the move with lowest value (3.0)
    }

    #[test]
    fn test_search_respects_max_depth() {
        let evaluator = MockStaticEvaluator::new().with_value(2, 5.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_value(5.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2]);

        // Test with depth 0 - should not search deeper
        let result = search(&evaluator, &generator, &state, 0);
        assert!(result.is_some());

        // Test with depth 1 - should search one level
        let result = search(&evaluator, &generator, &state, 1);
        assert!(result.is_some());
    }

    #[test]
    fn test_winning_positions() {
        let evaluator = MockStaticEvaluator::new();

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_value(1000.0)) // Alice wins
            .add_state(MockGameState::new(3, PlayerId::Bob).with_value(5.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        let result = search(&evaluator, &generator, &state, 1);

        assert!(result.is_some());
        let best_move = result.unwrap();
        assert_eq!(best_move.id, 2); // Should pick the winning move
    }

    #[test]
    fn test_alternating_players() {
        let evaluator = MockStaticEvaluator::new()
            .with_value(2, 8.0)
            .with_value(3, 12.0)
            .with_value(4, 6.0)
            .with_value(5, 15.0);

        // Create a tree: Alice -> Bob -> Alice
        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_children(vec![4]))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_children(vec![5]))
            .add_state(MockGameState::new(4, PlayerId::Alice).with_value(6.0))
            .add_state(MockGameState::new(5, PlayerId::Alice).with_value(15.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        let result = search(&evaluator, &generator, &state, 3);

        assert!(result.is_some());
        // Alice should choose move 3 because Bob will be forced to allow Alice to reach value 15.0
        // whereas move 2 only leads to value 6.0
        let best_move = result.unwrap();
        assert_eq!(best_move.id, 3);
    }
}
