//! Unit tests for the minimax search implementation
//!
//! These tests use mock implementations to verify the correctness of the minimax
//! algorithm, alpha-beta pruning, and transposition table integration.

use std::collections::HashMap;

use game_player::minimax::{ResponseGenerator, search};
use game_player::state::{PlayerId, State};
use game_player::static_evaluator::StaticEvaluator;

/// Mock action type for testing. `id` matches the target state's id for easy assertion.
#[derive(Debug, Clone, PartialEq)]
struct MockAction {
    id: u32,
    target_state: MockGameState,
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

    // Per the crate's policy (see minimax::ResponseGenerator::generate), a state is terminal if and only if its
    // response generator would return no actions - which, for MockResponseGenerator, happens exactly when `children`
    // is empty. `value` is unrelated to terminality: it is just a shortcut MockStaticEvaluator uses to read off a
    // leaf's static value directly instead of looking it up by id.
    fn is_terminal(&self) -> bool {
        self.children.is_empty()
    }

    fn apply(&self, action: &MockAction) -> Self {
        action.target_state.clone()
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
    fn generate(&self, state: &MockGameState, _depth: u32) -> Vec<MockAction> {
        state
            .children
            .iter()
            .filter_map(|&child_id| {
                self.states.get(&child_id).map(|child| MockAction {
                    id: child_id,
                    target_state: child.clone(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_returns_none_for_terminal_root() {
        let evaluator = MockStaticEvaluator::new();
        let generator = MockResponseGenerator::new();
        // No children => terminal (per the policy), so the generator correctly returns no actions.
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

        assert!(state1.is_terminal()); // No children: terminal by policy, regardless of `value`
        assert!(state2.is_terminal()); // No children: terminal (its `value` is just a static-eval shortcut)
        assert!(!state3.is_terminal()); // Has children: not terminal
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

    /// The greedy (depth-1) choice and the full minimax choice disagree.
    ///
    /// Tree:
    ///   Alice(1) → Bob(2) [SEF=10], Bob(3) [SEF=7]
    ///   Bob(2)   → Alice(4) [val=1], Alice(5) [val=2]   Bob picks min=1
    ///   Bob(3)   → Alice(6) [val=8], Alice(7) [val=6]   Bob picks min=6
    ///
    /// Depth-1: Alice sees 10 > 7, picks state 2.
    /// Full minimax: Alice sees max(1, 6) = 6, picks state 3.
    #[test]
    fn test_minimax_overrides_greedy_choice() {
        let evaluator = MockStaticEvaluator::new()
            .with_value(2, 10.0)
            .with_value(3, 7.0)
            .with_value(4, 1.0)
            .with_value(5, 2.0)
            .with_value(6, 8.0)
            .with_value(7, 6.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_children(vec![4, 5]))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_children(vec![6, 7]))
            .add_state(MockGameState::new(4, PlayerId::Alice).with_value(1.0))
            .add_state(MockGameState::new(5, PlayerId::Alice).with_value(2.0))
            .add_state(MockGameState::new(6, PlayerId::Alice).with_value(8.0))
            .add_state(MockGameState::new(7, PlayerId::Alice).with_value(6.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        // Shallow search is misled by the SEF and picks state 2 (value 10 > 7).
        let shallow = search(&evaluator, &generator, &state, 1);
        assert!(shallow.is_some());
        assert_eq!(shallow.unwrap().id, 2);

        // Full minimax correctly identifies state 3: Bob counters state 2 down to 1,
        // but can only hold state 3 to 6. Alice prefers 6 over 1.
        let deep = search(&evaluator, &generator, &state, 3);
        assert!(deep.is_some());
        assert_eq!(deep.unwrap().id, 3);
    }

    /// Alice avoids a move that lets Bob win, even though it looks good on the surface.
    ///
    /// Tree:
    ///   Alice(1) → Bob(2) [SEF=5], Bob(3) [SEF=3]
    ///   Bob(2)   → Alice(4) [val=-1000]   Bob wins
    ///   Bob(3)   → Alice(5) [val=5]
    ///
    /// Alice should pick state 3, not state 2.
    #[test]
    fn test_avoids_move_leading_to_bob_win() {
        let evaluator = MockStaticEvaluator::new().with_value(2, 5.0).with_value(3, 3.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_children(vec![4]))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_children(vec![5]))
            .add_state(MockGameState::new(4, PlayerId::Alice).with_value(-1000.0))
            .add_state(MockGameState::new(5, PlayerId::Alice).with_value(5.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        let result = search(&evaluator, &generator, &state, 3);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 3);
    }

    /// Bob seizes an immediately winning move.
    #[test]
    fn test_bob_picks_winning_move() {
        let evaluator = MockStaticEvaluator::new().with_value(2, 5.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Bob).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Alice).with_value(5.0))
            .add_state(MockGameState::new(3, PlayerId::Alice).with_value(-1000.0)); // Bob wins

        let state = MockGameState::new(1, PlayerId::Bob).with_children(vec![2, 3]);

        let result = search(&evaluator, &generator, &state, 1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 3);
    }

    /// The same leaf state is reachable from two different parents (diamond-shaped tree).
    /// The transposition table should cache state 4's value after the first visit and
    /// prevent a redundant recursive evaluation on the second visit.
    ///
    /// Tree:
    ///   Alice(1) → Bob(2), Bob(3)
    ///   Bob(2)   → Alice(4) [val=7]
    ///   Bob(3)   → Alice(4) [same fingerprint]
    #[test]
    fn test_transposition_table_shared_state() {
        let evaluator = MockStaticEvaluator::new().with_value(2, 6.0).with_value(3, 6.0);
        let shared_leaf = MockGameState::new(4, PlayerId::Alice).with_value(7.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_children(vec![4]))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_children(vec![4]))
            .add_state(shared_leaf);

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        // Both paths lead to value 7.0; the search must complete correctly.
        let result = search(&evaluator, &generator, &state, 3);
        assert!(result.is_some());
        let action = result.unwrap();
        assert!(action.id == 2 || action.id == 3);
    }

    /// Alpha-beta pruning fires but the returned move is still the correct minimax choice.
    ///
    /// Tree:
    ///   Alice(1) → Bob(2), Bob(3)       (sorted: Bob(2) SEF=8 first)
    ///   Bob(2)   → Alice(4)=8, Alice(5)=3   Bob picks min=3; Alice sets alpha=3
    ///   Bob(3)   → Alice(6)=1, Alice(7)=10  Bob evaluates 1 < alpha=3 → alpha cutoff
    ///
    /// Correct result: max(3, 1) = 3 → Alice picks state 2.
    /// State 7 must never influence the result (it is pruned).
    #[test]
    fn test_alpha_beta_pruning_correctness() {
        let evaluator = MockStaticEvaluator::new()
            .with_value(2, 8.0)
            .with_value(3, 6.0)
            .with_value(4, 8.0)
            .with_value(5, 3.0)
            .with_value(6, 1.0)
            .with_value(7, 10.0);

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]))
            .add_state(MockGameState::new(2, PlayerId::Bob).with_children(vec![4, 5]))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_children(vec![6, 7]))
            .add_state(MockGameState::new(4, PlayerId::Alice).with_value(8.0))
            .add_state(MockGameState::new(5, PlayerId::Alice).with_value(3.0))
            .add_state(MockGameState::new(6, PlayerId::Alice).with_value(1.0))
            .add_state(MockGameState::new(7, PlayerId::Alice).with_value(10.0));

        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2, 3]);

        // Bob(2) → min(8,3)=3; Bob(3) → alpha cutoff after seeing 1 < alpha=3
        // Alice: max(3, 1) = 3 → state 2
        let result = search(&evaluator, &generator, &state, 3);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 2);
    }

    /// A player with no real legal moves must still return an action (e.g. a "pass") rather than an empty vector,
    /// per the crate's no-legal-moves policy: an empty result is treated by the search as a definitive signal that
    /// the state is terminal. This builds a branch where Bob is forced to pass (his only legal action) into a
    /// state where Alice then has a real choice, and checks that the search recurses through the pass to find
    /// Alice's true best reply (9.0) rather than stopping there and trusting state 1's own, deliberately
    /// misleading, static value of -50.0.
    ///
    /// Tree:
    ///   Alice(0) → Bob(1) [SEF=-50, not terminal: forced to pass] → Alice(2) → leaves 9.0, 2.0
    ///   Alice(0) → Bob(3) [real choice]                                     → leaves 9.0, 1.0
    ///
    /// Via the pass branch, Alice ends up choosing between 9.0 and 2.0 herself (Bob had no say): 9.0.
    /// Via the real-choice branch, Bob minimizes between 9.0 and 1.0: 1.0.
    /// Alice must prefer the pass branch (9.0 > 1.0) - the opposite of what she'd pick if the search wrongly
    /// trusted state 1's raw static value of -50.0 instead of exploring the forced pass.
    #[test]
    fn test_forced_pass_is_explored_not_treated_as_terminal() {
        let evaluator = MockStaticEvaluator::new();

        let generator = MockResponseGenerator::new()
            .add_state(MockGameState::new(0, PlayerId::Alice).with_children(vec![1, 3]))
            .add_state(MockGameState::new(1, PlayerId::Bob).with_value(-50.0).with_children(vec![2])) // forced pass
            .add_state(MockGameState::new(2, PlayerId::Alice).with_children(vec![10, 11]))
            .add_state(MockGameState::new(10, PlayerId::Alice).with_value(9.0))
            .add_state(MockGameState::new(11, PlayerId::Alice).with_value(2.0))
            .add_state(MockGameState::new(3, PlayerId::Bob).with_children(vec![4, 5]))
            .add_state(MockGameState::new(4, PlayerId::Alice).with_value(9.0))
            .add_state(MockGameState::new(5, PlayerId::Alice).with_value(1.0));

        let state = MockGameState::new(0, PlayerId::Alice).with_children(vec![1, 3]);

        let result = search(&evaluator, &generator, &state, 4);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 1); // Alice prefers the forced-pass branch
    }

    /// A non-terminal state whose generator (in violation of the crate's no-legal-moves policy) returns no actions
    /// must be caught immediately by the search's debug assertion, rather than silently mistreated as terminal.
    /// State 1 declares a child (so it is not terminal), but that child was never registered with the generator, so
    /// `generate` resolves to an empty vector for it.
    #[test]
    #[should_panic(expected = "ResponseGenerator::generate must return no actions if and only if the state is terminal")]
    fn test_policy_violation_panics_in_debug_builds() {
        let evaluator = MockStaticEvaluator::new();
        let generator = MockResponseGenerator::new(); // state 2 is never registered
        let state = MockGameState::new(1, PlayerId::Alice).with_children(vec![2]);

        let _ = search(&evaluator, &generator, &state, 2);
    }
}
