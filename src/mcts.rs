//! Monte Carlo Tree Search (MCTS) module
//!
//! This module implements the Monte Carlo Tree Search (MCTS) algorithm for making decisions in two-player perfect and hidden
//! information games. It provides the core MCTS logic, including selection, expansion, simulation (rollout), and back-propagation
//! phases. The MCTS algorithm builds a search tree incrementally and uses random simulations to evaluate the potential of
//! different moves. The module is designed to be generic and works with any game state that implements the `State` trait, along
//! with the `ResponseGenerator` and `Rollout` traits.

use crate::state::State;
use indextree::{Arena, NodeId};

/// Default exploration constant for the UCT formula
pub const DEFAULT_EXPLORATION_CONSTANT: f32 = 1.4142135623730951; // sqrt(2)

/// Response generator trait for MCTS search
pub trait ResponseGenerator {
    /// The type representing game states that this generator works with
    type State: State;

    /// Generates a list of all possible legal actions from the given state.
    ///
    /// # Arguments
    /// * `state` - state to respond to
    ///
    /// # Returns
    /// List of all possible legal actions for the given state.
    ///
    /// # Notes
    /// - All returned actions must be legal in the provided state.
    /// - The order of actions is not significant unless required by the implementation.
    /// - Returning no actions indicates that the player cannot respond. It does not necessarily indicate that the game is
    ///   over or that the player has passed. If passing is allowed, then a pass must be a valid action.
    fn generate(&self, state: &Self::State) -> Vec<<Self::State as State>::Action>;
}

/// Rollout trait for MCTS search
///
/// The Rollout trait defines the interface for performing rollouts in a game state. It is used by the MCTS algorithm to simulate
/// game play and evaluate the potential outcomes of different moves. It is implemented for specific game states in the
/// game-specific code.
///
/// # Type Parameters
/// * `S` - Game state type
///
/// # Examples
///
/// ```rust
/// # use hidden_game_player::mcts::Rollout;
/// # use hidden_game_player::{State, StaticEvaluator};
/// # #[derive(Debug, Clone, Default)]
/// # struct TestGameState { value: i32 }
/// # impl State for TestGameState {
/// #     type Action = TestAction;
/// #     fn fingerprint(&self) -> u64 { self.value as u64 }
/// #     fn whose_turn(&self) -> u8 { 0 }
/// #     fn is_terminal(&self) -> bool { false }
/// #     fn apply(&self, _action: &TestAction) -> Self { self.clone() }
/// # }
/// #[derive(Debug, Clone, Default)]
/// struct TestAction;
/// struct TestRollout;
/// impl Rollout for TestRollout {
///     type State = TestGameState;
///     fn play(&self, state: &TestGameState) -> f32 { state.value as f32 }
/// }
/// let state = TestGameState { value: 42 };
/// let rollout = TestRollout;
/// let score = rollout.play(&state);
/// assert_eq!(score, 42.0);
/// ```
pub trait Rollout {
    /// The type representing the game state
    type State: State;
    type ResponseGenerator: ResponseGenerator<State = Self::State>;

    /// Performs a rollout (simulation) from the given state using the provided response generator, and returns the evaluated value.
    ///
    /// The rollout is a simulation of the game from the given state to a terminal state, following simple heuristics or random
    /// moves. The result is a score in the range [-1.0, 1.0], where 1.0 indicates a win for the computer player, -1.0 indicates a
    /// loss, and 0.0 indicates a draw.
    ///
    /// # Arguments
    /// * `state` - The game state to perform the rollout from
    /// * `response_generator` - The response generator for producing legal actions
    ///
    /// # Returns
    /// [-1.0, 1.0] score representing the outcome of the rollout from the perspective of the computer player.
    fn play(&self, state: &Self::State, response_generator: &Self::ResponseGenerator) -> f32;
}

/// Represents a node in the MCTS tree
///
/// # Type Parameters
/// * `S` - Game state type
///
/// # Examples
///
/// ```rust
/// # use hidden_game_player::mcts::ResponseGenerator;
/// # use hidden_game_player::{State, StaticEvaluator};
/// # use indextree::{Arena, NodeId};
///
/// # #[derive(Debug, Clone, Default)]
/// # struct TestGameState { value: i32 }
/// # impl State for TestGameState {
/// #     type Action = TestAction;
/// #     fn fingerprint(&self) -> u64 { self.value as u64 }
/// #     fn whose_turn(&self) -> u8 { 0 }
/// #     fn is_terminal(&self) -> bool { false }
/// #     fn apply(&self, _action: &TestAction) -> Self { self.clone() }
/// # }
/// # #[derive(Debug, Clone, Default)]
/// # struct TestAction;
/// # struct TestResponseGen;
/// # impl ResponseGenerator for TestResponseGen {
/// #     type State = TestGameState;
/// #     fn generate(&self, _state: &TestGameState) -> Vec<TestAction> { vec![TestAction] }
/// # }
///
/// let state = TestGameState { value: 42 };
/// let response_gen = TestResponseGen;
/// // This example shows basic usage of the test types
/// assert_eq!(state.value, 42);
/// ```
/// ```
struct Node<S>
where
    S: State,
{
    /// The game state represented by this node
    state: S,
    /// Action that led to this node
    action: Option<S::Action>,
    /// Untried actions that have not been expanded yet
    untried_actions: Vec<S::Action>,
    /// Number of times this node has been visited
    visits: u32,
    /// Sum of the values of all simulations that passed through this node
    value_sum: f32,
}

impl<S> Node<S>
where
    S: State,
{
    // Creates a new node with the given game state and action
    //
    // # Arguments
    // * `state` - The game state this node represents
    // * `action` - The action that led to this state from the parent, None for the root
    // * `rg` - Response generator to determine possible actions from this state
    //
    // # Returns
    // A new Node instance with zero visits
    fn new<G>(state: S, action: Option<S::Action>, rg: &G) -> Self
    where
        G: ResponseGenerator<State = S>,
    {
        let untried_actions = rg.generate(&state);
        Self {
            state,
            action,
            untried_actions,
            visits: 0,
            value_sum: 0.0,
        }
    }

    // Checks if the node is fully expanded
    //
    // A node is fully expanded when all possible actions from this state have been
    // tried and added as child nodes.
    //
    // # Returns
    // `true` if no untried actions remain, `false` otherwise
    fn fully_expanded(&self) -> bool {
        self.untried_actions.is_empty()
    }

    // Calculates the UCT value for this node
    //
    // The UCT formula balances exploitation (average reward) with exploration (uncertainty).
    // Higher UCT values indicate more promising nodes to explore.
    //
    // # Arguments
    // * `arena` - The Arena containing all nodes
    // * `c` - Exploration constant (typically sqrt(2) ≈ 1.414)
    //
    // # Panics
    // Panics if the parent node does not exist or has zero visits.
    //
    // # Returns
    // The UCT value for this node, or f32::INFINITY if unvisited
    fn uct(&self, node_id: NodeId, arena: &Arena<Node<S>>, c: f32) -> f32 {
        if let Some(parent_node) = arena[node_id].parent().and_then(|parent_id| arena.get(parent_id)) {
            // If this node has never been visited, return infinity to ensure it gets visited
            if self.visits == 0 {
                return f32::INFINITY;
            }
            let parent_visits = parent_node.get().visits;
            if parent_visits > 0 {
                let confidence = c * ((parent_visits as f32).ln() / self.visits as f32).sqrt();
                let mean_value = self.value_sum / self.visits as f32;
                return mean_value + confidence
            }
        }

        panic!("UCT cannot be computed because the parent node does not exist or has no visits");
        f32::INFINITY
    }
}

// Holds static information for the MCTS search
struct Context<'a, G, R>
where
    G: ResponseGenerator,
    R: Rollout<State = G::State>,
{
    /// Function to generate all possible child states
    response_generator: &'a G,
    /// Rollout implementation
    rollout: &'a R,
    /// Exploration constant for the UCT formula
    c: f32,
}

/// Searches for the best action using the MCTS algorithm
///
/// Performs the four phases of MCTS (Selection, Expansion, Rollout, Back Propagation) for the given number of iterations,
/// building up statistics in the search tree.
///
/// # Arguments
/// * `s0` - Initial game state to serve as the root of the search tree
/// * `rg` - Response generator that returns all possible actions from a state
/// * `roll` - Rollout implementation for simulating games
/// * `c` - Exploration constant for UCT calculation
/// * `max_iterations` - Number of MCTS iterations to perform
///
/// # Returns
/// Some(best_action) containing the action leading to the child of the root node with the most visits (most promising move),
/// or None if the root state has no possible actions.
///
/// # Panics
/// This function will panic if the UCT function ever returns NaN.
pub fn search<S, G, R>(
    s0: &S,
    rg: &G,
    roll: &R,
    c: f32,
    max_iterations: u32,
) -> Option<S::Action>
where
    S: State + Clone,
    G: ResponseGenerator<State = S>,
    R: Rollout<State = S, ResponseGenerator = G>,
{
    // Create context for the search
    let context = Context {
        response_generator: rg,
        rollout: roll,
        c,
    };

    // Create the arena that will hold all nodes
    let mut arena = Arena::new();

    // Initialize root node
    let root_node = Node::new(s0.clone(), None, rg);
    let root_id = arena.new_node(root_node);
    arena.get_mut(root_id).unwrap().get_mut().visits = 1; // Root node is automatically visited once

    for _ in 0..max_iterations {
        // Selection - traverse the tree to find the best leaf node to expand
        let mut node_id = select(root_id, &arena, &context);

        // Expansion - add another child to the node if it is not terminal and has untried actions
        if let Some(child_id) = expand(node_id, &mut arena, &context) {
            node_id = child_id;
        }

        // Rollout - evaluate the node using the Rollout implementation, and normalize the result to [0, 1]
        let value = (rollout(node_id, &arena, &context) + 1.0) / 2.0;

        // Back-propagation - update the node and its ancestors with the rollout result
        back_propagate(node_id, &mut arena, value);
    }

    // If there are no responses to the root state then return None
    if root_id.children(&arena).count() == 0 {
        return None;
    }

    // Get the best child (of root) by the number of visits, or None if there are no children
    let best_child_id = root_id.children(&arena)
        .max_by(|&a, &b| {
            let a_visits = arena[a].get().visits;
            let b_visits = arena[b].get().visits;
            a_visits.cmp(&b_visits)
        });

    // Return the action that led to the best child
    best_child_id.and_then(|child_id| arena[child_id].get().action.clone())
}

// Helper function that determines if the node should be selected for expansion.
// A node is selectable if it is not fully expanded, has no children, or represents a terminal game state.
fn selectable<S>(node_id: NodeId, arena: &Arena<Node<S>>) -> bool
where
    S: State,
{
    let has_children = node_id.children(arena).count() > 0;
    let node = arena[node_id].get();
    !node.fully_expanded() || !has_children || node.state.is_terminal()
}

// Selects the best node for expansion using the UCT value
//
// This method traverses the tree from the given node downward, selecting the child with the highest UCT value at each step
// until it reaches a node that is either not fully expanded, has no children, or represents a terminal game state. That node is
// returned.
//
// # Arguments
// * `node_id` - The starting node for selection (typically the root)
// * `arena` - The Arena containing all nodes
// * `context` - The search context containing parameters
//
// # Returns
// The node selected for expansion or evaluation
fn select<G, R>(node_id: NodeId, arena: &Arena<Node<G::State>>, context: &Context<'_, G, R>) -> NodeId
where
    G: ResponseGenerator,
    R: Rollout<State = G::State>,
{
    let c = context.c;
    let mut selected = node_id;

    // Traverse the tree until a selectable node is found
    // If a node is not fully expanded, then select it for expansion.
    // If a node is terminal or has no children, then select it for evaluation.
    // Otherwise, descend to the child with the highest UCT value and continue.
    while !selectable(selected, arena) {
        let children: Vec<NodeId> = selected.children(arena).collect();
        let best_child = children.iter()
                .max_by(|&a, &b| {
                    let a_uct = arena.get(*a).unwrap().get().uct(*a, arena, c);
                    let b_uct = arena.get(*b).unwrap().get().uct(*b, arena, c);
                    a_uct.total_cmp(&b_uct)
                })
                .unwrap() // Safe to unwrap because not_selectable ensures there are children
                .clone();
        selected = best_child;
    }

    selected
}

// Expands a node by adding a child for one of its untried actions and returns the new child node
//
// If the node is fully expanded (no untried actions remain) or represents a terminal game state, this function returns None.
//
// # Arguments
// * `node_id` - The node to expand
// * `arena` - The arena containing all nodes
// * `context` - The search context containing the response generator
//
// # Returns
// Some(child_node_id) if expansion was successful, None otherwise
fn expand<G, R>(node_id: NodeId, arena: &mut Arena<Node<G::State>>, context: &Context<'_, G, R>) -> Option<NodeId>
where
    G: ResponseGenerator,
    R: Rollout<State = G::State>,
{
    // Get the next untried action, or return None if there are no untried actions
    let action = arena[node_id].get_mut().untried_actions.pop()?;

    // Apply the action to the node's state to get a new state and create a new child node
    let child_state = arena[node_id].get().state.apply(&action);

    // Create the new child node and add it to the arena
    let child_node = Node::new(child_state, Some(action), context.response_generator);
    let child_id = arena.new_node(child_node);

    // Add the new child to the parent node using indextree's append
    node_id.append(child_id, arena);

    // Return the new child node ID
    Some(child_id)
}

// Performs a rollout (simulation) from the given node and returns the evaluated value
//
// In the current implementation, rollout simply evaluates the current node's state using the static evaluator rather than
// performing random simulation.
//
// # Arguments
// * `node_id` - The node to evaluate
// * `arena` - The arena containing all nodes
// * `context` - The search context containing the rollout implementation
//
// # Returns
// [-1.0, 1.0] as the evaluation score for the node's game state
fn rollout<G, R>(node_id: NodeId, arena: &Arena<Node<G::State>>, context: &Context<'_, G, R>) -> f32
where
    G: ResponseGenerator,
    R: Rollout<State = G::State, ResponseGenerator = G>,
{
    context.rollout.play(&arena[node_id].get().state, &context.response_generator)
}

// Back-propagates the value up the tree
//
// Updates the visit count and value sum for the given node and all of its ancestors up to the root.
//
// # Arguments
// * `node_id` - The starting node for back-propagation
// * `arena` - The arena containing all nodes
// * `value` - The value to propagate up the tree
fn back_propagate<S>(node_id: NodeId, arena: &mut Arena<Node<S>>, value: f32)
where
    S: State,
{
    let mut current = Some(node_id);
    while let Some(id) = current {
        let node = arena[id].get_mut();
        node.visits += 1;
        node.value_sum += value;
        current = arena[id].parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementations for testing
    #[derive(Debug, Clone, Default, PartialEq)]
    struct TestGameState {
        value: i32,
        terminal: bool,
    }

    impl State for TestGameState {
        type Action = TestAction;

        fn fingerprint(&self) -> u64 {
            self.value as u64
        }

        fn whose_turn(&self) -> u8 {
            0
        }

        fn is_terminal(&self) -> bool {
            self.terminal
        }

        fn apply(&self, action: &TestAction) -> Self {
            Self {
                value: self.value + action.increment,
                terminal: self.value + action.increment > 10,
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    struct TestAction {
        increment: i32,
    }

    impl TestAction {
        fn new(increment: i32) -> Self {
            Self { increment }
        }
    }

    struct TestResponseGenerator;

    impl ResponseGenerator for TestResponseGenerator {
        type State = TestGameState;

        fn generate(&self, state: &TestGameState) -> Vec<TestAction> {
            if state.terminal {
                vec![]
            } else {
                vec![TestAction::new(1), TestAction::new(2)]
            }
        }
    }

    struct EmptyResponseGenerator;

    impl ResponseGenerator for EmptyResponseGenerator {
        type State = TestGameState;

        fn generate(&self, _state: &TestGameState) -> Vec<TestAction> {
            vec![]
        }
    }

    struct SingleResponseGenerator;

    impl ResponseGenerator for SingleResponseGenerator {
        type State = TestGameState;

        fn generate(&self, state: &TestGameState) -> Vec<TestAction> {
            if state.terminal {
                vec![]
            } else {
                vec![TestAction::new(1)]
            }
        }
    }

    struct VariableResponseGenerator;

    impl ResponseGenerator for VariableResponseGenerator {
        type State = TestGameState;

        fn generate(&self, state: &TestGameState) -> Vec<TestAction> {
            if state.terminal {
                vec![]
            } else if state.value < 3 {
                vec![TestAction::new(1), TestAction::new(2), TestAction::new(3)]
            } else if state.value < 6 {
                vec![TestAction::new(1), TestAction::new(2)]
            } else {
                vec![TestAction::new(1)]
            }
        }
    }

    struct TestRollout;

    impl Rollout for TestRollout {
        type State = TestGameState;
        type ResponseGenerator = TestResponseGenerator;

        fn play(&self, _state: &TestGameState, _rg: &TestResponseGenerator) -> f32 {
            0.5 // Simple fixed rollout value for testing
        }
    }

    // Tests for ResponseGenerator trait
    #[test]
    fn test_mcts_response_generator_basic() {
        let generator = TestResponseGenerator;
        let state = TestGameState {
            value: 5,
            terminal: false,
        };
        let terminal_state = TestGameState {
            value: 15,
            terminal: true,
        };

        let actions = generator.generate(&state);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].increment, 1);
        assert_eq!(actions[1].increment, 2);

        let terminal_actions = generator.generate(&terminal_state);
        assert!(terminal_actions.is_empty());
    }

    #[test]
    fn test_mcts_response_generator_empty() {
        let generator = EmptyResponseGenerator;
        let state = TestGameState {
            value: 0,
            terminal: false,
        };

        let actions = generator.generate(&state);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_mcts_response_generator_single() {
        let generator = SingleResponseGenerator;
        let state = TestGameState {
            value: 3,
            terminal: false,
        };

        let actions = generator.generate(&state);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].increment, 1);
    }

    #[test]
    fn test_mcts_response_generator_variable() {
        let generator = VariableResponseGenerator;

        // Low value state should have 3 actions
        let low_state = TestGameState {
            value: 1,
            terminal: false,
        };
        let actions = generator.generate(&low_state);
        assert_eq!(actions.len(), 3);

        // Medium value state should have 2 actions
        let med_state = TestGameState {
            value: 4,
            terminal: false,
        };
        let actions = generator.generate(&med_state);
        assert_eq!(actions.len(), 2);

        // High value state should have 1 action
        let high_state = TestGameState {
            value: 7,
            terminal: false,
        };
        let actions = generator.generate(&high_state);
        assert_eq!(actions.len(), 1);

        // Terminal state should have no actions
        let terminal_state = TestGameState {
            value: 15,
            terminal: true,
        };
        let actions = generator.generate(&terminal_state);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_mcts_search_basic() {
        let state = TestGameState { value: 0, terminal: false };
        let generator = TestResponseGenerator;
        let rollout = TestRollout;

        // Test with minimal iterations
        let result = search(&state, &generator, &rollout, 1.0, 1);
        // Since we have actions available, should return Some action
        assert!(result.is_some());
    }

    #[test]
    fn test_mcts_search_terminal_state() {
        let state = TestGameState { value: 0, terminal: true };
        let generator = TestResponseGenerator;
        let rollout = TestRollout;

        // Terminal state should return None (no actions)
        let result = search(&state, &generator, &rollout, 1.0, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_mcts_search_zero_iterations() {
        let state = TestGameState { value: 0, terminal: false };
        let generator = TestResponseGenerator;
        let rollout = TestRollout;

        // Zero iterations should still work
        let result = search(&state, &generator, &rollout, 1.0, 0);
        // Might return None or Some depending on implementation
        // Just verify it doesn't crash
        let _ = result;
    }

    #[test]
    fn test_mcts_search_different_c_values() {
        let state = TestGameState { value: 0, terminal: false };
        let generator = TestResponseGenerator;
        let rollout = TestRollout;

        // Test with different exploration constants
        let result1 = search(&state, &generator, &rollout, 0.1, 5);
        let result2 = search(&state, &generator, &rollout, 2.0, 5);

        // Both should work (might return different results)
        assert!(result1.is_some() || state.terminal);
        assert!(result2.is_some() || state.terminal);
    }

    #[test]
    fn test_mcts_search_multiple_iterations() {
        let state = TestGameState { value: 0, terminal: false };
        let generator = TestResponseGenerator;
        let rollout = TestRollout;

        // Test with multiple iterations
        let result = search(&state, &generator, &rollout, 1.4, 50);

        // Should return an action if state is not terminal
        if !state.terminal {
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_mcts_search_consistency() {
        let state = TestGameState { value: 5, terminal: false };
        let generator = TestResponseGenerator;
        let rollout = TestRollout;

        // Multiple searches on same state should work
        let result1 = search(&state, &generator, &rollout, 1.0, 10);
        let result2 = search(&state, &generator, &rollout, 1.0, 10);

        // Both should return results (might be different due to randomness)
        assert!(result1.is_some());
        assert!(result2.is_some());
    }
}