//! Monte Carlo Tree Search (MCTS) module
//!
//! This module implements the Monte Carlo Tree Search (MCTS) algorithm for making decisions in two-player, perfect-information
//! games. It builds a search tree incrementally through four phases: Selection, Expansion, Evaluation, and Back-propagation.
//! The Evaluation phase runs a `ValueEstimator`, which returns a value in `[0.0, 1.0]` from the perspective of the current
//! player (`state.whose_turn()`), with terminal states returning the exact outcome (0.0 loss, 1.0 win, 0.5 draw); random
//! playout to a terminal state is one possible estimator strategy among others, such as a static evaluation function or a
//! neural network. The module is designed to be generic and works with any game state that implements the `State` trait,
//! along with the `ResponseGenerator` and `ValueEstimator` traits.
//!
//! Node statistics (`value_sum`, `initial_value`) are stored from the perspective of the player who chose the action leading
//! into the node — that is, the `whose_turn()` of the parent node's state, meaning "wins for the player who just moved". This
//! convention is why Selection is a plain argmax of UCT at every level of the tree, and why the search remains correct for
//! adversarial two-player play; perspective comparisons throughout use `whose_turn()` equality, never ply parity.
//!
//! Two knobs tune the search. `initial_value_weight` (`w`) blends a node's stored initial estimate into UCT as `w` virtual
//! visits: with `n_eff = visits + w` and `Q = (value_sum + w * initial_value) / n_eff`, `w = 0` reduces to the classic UCT
//! formula. `estimate_on_expansion` selects lazy expansion (`false`: one child created and estimated per iteration) versus
//! eager expansion (`true`: all remaining children created and estimated at once, one estimator call per child, with only
//! the chosen child's estimate back-propagated) — eager suits cheap estimators such as static evaluation, since it costs
//! branching-factor-times more estimator calls per expansion than lazy expansion.
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::mcts::{search, ResponseGenerator, ValueEstimator, DEFAULT_EXPLORATION_CONSTANT, DEFAULT_INITIAL_VALUE_WEIGHT};
//!
//! // Assuming you have implemented the required traits for your game
//! let response_generator = MyResponseGenerator::new();
//! let estimator = MyValueEstimator::new();
//! let initial_state = MyGameState::new();
//!
//! if let Some(action) = search(
//!     &initial_state,
//!     &response_generator,
//!     &estimator,
//!     DEFAULT_EXPLORATION_CONSTANT,
//!     DEFAULT_INITIAL_VALUE_WEIGHT,
//!     false, // lazy expansion
//!     1000,  // iterations
//! ) {
//!     println!("Best action found: {:?}", action);
//!     let next_state = initial_state.apply(&action);
//! }
//! ```
//!
//! # Notes
//! - The search assumes a two-player game with perfect information; adversarial play is captured entirely through
//!   `whose_turn()`, so it is correct even for games where a player may move twice in a row.
//! - Unlike `minimax::search`, which takes a `max_depth`, MCTS spends a fixed `max_iterations` budget regardless of
//!   how deep any particular line goes.
//! - No transposition table is used: states reached by different move orders are treated as distinct nodes.

use crate::state::*;
use indextree::{Arena, NodeId};

/// Default exploration constant for the UCT formula
pub const DEFAULT_EXPLORATION_CONSTANT: f32 = std::f32::consts::SQRT_2;

/// Default weight given to a node's initial value estimate in the UCT formula
pub const DEFAULT_INITIAL_VALUE_WEIGHT: f32 = 0.0;

/// Response generator trait for MCTS search
///
/// This trait defines the interface for generating all possible responses from a given state, for use during the
/// MCTS Expansion phase. It is distinct from [`minimax::ResponseGenerator`](crate::minimax::ResponseGenerator): its
/// `generate` method takes no `depth` parameter, since MCTS does not track a fixed search depth the way minimax
/// does.
///
/// # Examples
/// ```rust,ignore
/// use crate::mcts::ResponseGenerator;
///
/// struct MyResponseGenerator;
///
/// impl ResponseGenerator for MyResponseGenerator {
///     type State = MyGameState;
///
///     fn generate(&self, state: &Self::State) -> Vec<<Self::State as State>::Action> {
///         // Enumerate all legal actions for the current player
///         get_all_valid_moves(state)
///     }
/// }
/// ```
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

/// Value estimator trait for MCTS search
///
/// Provides the evaluation used in the MCTS Evaluation phase. Implementations may use
/// any strategy: random playout to a terminal state (the classic MCTS rollout), a
/// static evaluation function, a neural network, etc.
///
/// # Examples
///
/// ```rust
/// # use game_player::mcts::{ValueEstimator, ResponseGenerator};
/// # use game_player::state::*;
/// # #[derive(Debug, Clone, Default)]
/// # struct TestGameState { value: i32 }
/// # impl State for TestGameState {
/// #     type Action = TestAction;
/// #     fn fingerprint(&self) -> u64 { self.value as u64 }
/// #     fn whose_turn(&self) -> PlayerId { PlayerId::Alice }
/// #     fn is_terminal(&self) -> bool { false }
/// #     fn apply(&self, _action: &TestAction) -> Self { self.clone() }
/// # }
/// #[derive(Debug, Clone, Default)]
/// struct TestAction;
/// struct TestResponseGen;
/// impl ResponseGenerator for TestResponseGen {
///     type State = TestGameState;
///     fn generate(&self, _state: &TestGameState) -> Vec<TestAction> { vec![TestAction] }
/// }
/// struct ConstantEstimator;
/// impl ValueEstimator for ConstantEstimator {
///     type State = TestGameState;
///     type ResponseGenerator = TestResponseGen;
///     fn estimate(&self, _state: &TestGameState, _rg: &TestResponseGen) -> f32 { 0.5 }
/// }
/// let state = TestGameState { value: 42 };
/// let estimator = ConstantEstimator;
/// let rg = TestResponseGen;
/// assert_eq!(estimator.estimate(&state, &rg), 0.5);
/// ```
pub trait ValueEstimator {
    /// The type representing the game state
    type State: State;
    /// The response generator type usable by playout-based estimators
    type ResponseGenerator: ResponseGenerator<State = Self::State>;

    /// Returns an estimate of the state's value in [0.0, 1.0] from the perspective of the
    /// current player (`state.whose_turn()`): 0.0 is a terminal loss and 1.0 a terminal win
    /// for that player; a draw is 0.5. For terminal states the returned value must be the
    /// exact outcome. Implementations may use any strategy: random playout to a terminal
    /// state (the classic MCTS rollout), a static evaluation function, a neural network, etc.
    ///
    /// # Arguments
    /// * `state` - The game state to estimate the value of
    /// * `rg` - Response generator for producing legal actions (used by playout-based
    ///   estimators; others may ignore it)
    ///
    /// # Returns
    /// [0.0, 1.0] value estimate from the perspective of `state.whose_turn()`.
    ///
    /// # Adapting a StaticEvaluator
    /// An Alice-perspective `StaticEvaluator` can be wrapped to implement this trait by
    /// normalizing its output into `[0.0, 1.0]` and then flipping perspective for Bob: compute
    /// `v01 = (eval - bob_wins_value()) / (alice_wins_value() - bob_wins_value())`, then return
    /// `v01` when `state.whose_turn()` is Alice, or `1.0 - v01` when it is Bob.
    fn estimate(&self, state: &Self::State, rg: &Self::ResponseGenerator) -> f32;
}

/// Represents a node in the MCTS tree
///
/// # Type Parameters
/// * `S` - Game state type
///
/// # Examples
///
/// ```rust
/// # use game_player::mcts::ResponseGenerator;
/// # use game_player::state::*;
///
/// # #[derive(Debug, Clone, Default)]
/// # struct TestGameState { value: i32 }
/// # impl State for TestGameState {
/// #     type Action = TestAction;
/// #     fn fingerprint(&self) -> u64 { self.value as u64 }
/// #     fn whose_turn(&self) -> PlayerId { PlayerId::Alice }
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
    /// Initial value estimate for this node, stored from the perspective of the player
    /// who chose the action leading into it (None until an estimate is recorded)
    initial_value: Option<f32>,
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
            initial_value: None,
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

    // Calculates the UCT value for this node, blended with its initial value estimate
    //
    // The UCT formula balances exploitation (average reward) with exploration (uncertainty).
    // Higher UCT values indicate more promising nodes to explore.
    //
    // With `w = initial_value_weight` and `v0 = self.initial_value`, the node's initial value
    // is treated as `w` virtual visits: `n_eff = visits + w`, `Q = (value_sum + w*v0) / n_eff`,
    // and `UCT = Q + c * sqrt(ln(parent_visits) / n_eff)`. The weight `w` only counts when
    // `self.initial_value` is `Some` and `initial_value_weight > 0.0`; otherwise it is treated
    // as `0.0`, which reduces the formula bit-for-bit to the legacy (unweighted) UCT.
    //
    // # Arguments
    // * `arena` - The Arena containing all nodes
    // * `c` - Exploration constant (typically sqrt(2) ≈ 1.414)
    // * `initial_value_weight` - Virtual-visit weight given to `self.initial_value`
    //
    // # Panics
    // Panics if the parent node does not exist or has zero visits.
    //
    // # Returns
    // The UCT value for this node, or f32::INFINITY if unvisited and no usable initial value
    // estimate is present (i.e. `initial_value` is `None` or `initial_value_weight <= 0.0`)
    fn uct(&self, node_id: NodeId, arena: &Arena<Node<S>>, c: f32, initial_value_weight: f32) -> f32 {
        if let Some(parent_node) = arena[node_id].parent().and_then(|parent_id| arena.get(parent_id)) {
            // The initial-value weight counts only when an estimate is present
            let w = match self.initial_value {
                Some(_) if initial_value_weight > 0.0 => initial_value_weight,
                _ => 0.0,
            };
            // Never-visited node with no usable estimate: force a visit
            if self.visits == 0 && w == 0.0 {
                return f32::INFINITY;
            }
            let parent_visits = parent_node.get().visits;
            if parent_visits > 0 {
                let v0 = self.initial_value.unwrap_or(0.0);
                let n_eff = self.visits as f32 + w;
                let q = (self.value_sum + w * v0) / n_eff;
                let confidence = c * ((parent_visits as f32).ln() / n_eff).sqrt();
                return q + confidence;
            }
        }

        panic!("UCT cannot be computed because the parent node does not exist or has no visits");
    }
}

// Holds static information for the MCTS search
struct Context<'a, G, E>
where
    G: ResponseGenerator,
    E: ValueEstimator<State = G::State>,
{
    /// Function to generate all possible child states
    response_generator: &'a G,
    /// Value estimator implementation
    estimator: &'a E,
    /// Exploration constant for the UCT formula
    c: f32,
    /// Weight of a node's initial value estimate in the UCT formula
    initial_value_weight: f32,
    /// If true, estimate all children when a node is expanded
    estimate_on_expansion: bool,
}

/// Searches for the best action using the MCTS algorithm
///
/// Performs the four phases of MCTS (Selection, Expansion, Evaluation, Back Propagation) for the given number of iterations,
/// building up statistics in the search tree.
///
/// # Arguments
/// * `s0` - Initial game state to serve as the root of the search tree
/// * `rg` - Response generator that returns all possible actions from a state
/// * `estimator` - Value estimator implementation for evaluating leaf states
/// * `exploration_constant` - Exploration constant for UCT calculation
/// * `initial_value_weight` - Weight of a node's initial value estimate in the UCT formula
/// * `estimate_on_expansion` - If true, estimate all children when a node is expanded (eager
///   expansion): every untried child is created and estimated at once, at a cost of
///   branching-factor-times more estimator calls per expansion, so it suits cheap static
///   evaluators rather than expensive playouts. Combining eager expansion with
///   `initial_value_weight = 0` wastes the stored estimates, since unvisited children then
///   fall back to `f32::INFINITY` in UCT and are chosen in arbitrary first-visit order.
/// * `max_iterations` - Number of MCTS iterations to perform
///
/// # Returns
/// Some(best_action) containing the action leading to the child of the root node with the most visits (most promising move),
/// or None if the root state has no possible actions.
///
/// # Panics
/// This function will panic if the UCT function ever returns NaN.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::mcts::{search, DEFAULT_EXPLORATION_CONSTANT, DEFAULT_INITIAL_VALUE_WEIGHT};
///
/// // Set up the search components
/// let move_generator = MyResponseGenerator::new();
/// let estimator = MyValueEstimator::new();
/// let game_state = MyGameState::initial_position();
///
/// // Search for 1000 iterations
/// match search(
///     &game_state,
///     &move_generator,
///     &estimator,
///     DEFAULT_EXPLORATION_CONSTANT,
///     DEFAULT_INITIAL_VALUE_WEIGHT,
///     false, // lazy expansion
///     1000,
/// ) {
///     Some(action) => {
///         println!("Best action: {:?}", action);
///         let next_state = game_state.apply(&action);
///     }
///     None => println!("No moves available"),
/// }
/// ```
///
/// # Algorithm Details
///
/// The search repeats four phases each iteration:
/// - **Selection**: Descend from the root by a plain argmax of UCT at every level, stopping at a node that is not
///   fully expanded, has no children, or is terminal.
/// - **Expansion**: Add one untried child (lazy, the default), or every untried child at once (eager, when
///   `estimate_on_expansion` is `true`).
/// - **Evaluation**: Run the `ValueEstimator` on the newly expanded (or terminal) state.
/// - **Back-propagation**: Credit the evaluation to the node and all of its ancestors, flipping perspective wherever
///   `whose_turn()` differs from the leaf's current player.
///
/// Unlike `minimax::search`, there is no alternating max/min and no transposition table; adversarial correctness
/// comes entirely from the perspective bookkeeping described in the module docs.
pub fn search<S, G, E>(
    s0: &S,
    rg: &G,
    estimator: &E,
    exploration_constant: f32,
    initial_value_weight: f32,
    estimate_on_expansion: bool,
    max_iterations: u32,
) -> Option<S::Action>
where
    S: State + Clone,
    G: ResponseGenerator<State = S>,
    E: ValueEstimator<State = S, ResponseGenerator = G>,
{
    // Create context for the search
    let context = Context {
        response_generator: rg,
        estimator,
        c: exploration_constant,
        initial_value_weight,
        estimate_on_expansion,
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

        let value = if context.estimate_on_expansion {
            // Eager expansion - create every remaining child at once; the estimator
            // already ran on the chosen child, so its raw estimate back-propagates
            // with no second call. Sibling estimates are stored, never back-propagated.
            if let Some((child_id, raw_estimate)) = expand_eager(node_id, &mut arena, &context) {
                node_id = child_id;
                raw_estimate
            } else {
                // Terminal or childless node - evaluate it directly
                estimate_leaf(node_id, &arena, &context)
            }
        } else {
            // Expansion - add another child to the node if it is not terminal and has untried actions
            let expanded = expand(node_id, &mut arena, &context);
            if let Some(child_id) = expanded {
                node_id = child_id;
            }

            // Evaluation - evaluate the node using the ValueEstimator implementation
            let value = estimate_leaf(node_id, &arena, &context);

            // Record a newly expanded child's initial value, adjusted to the perspective of
            // the player who chose the action leading into it
            if expanded.is_some() {
                let parent_id = arena[node_id].parent().expect("expanded child has a parent");
                let parent_player = arena[parent_id].get().state.whose_turn();
                let child_player = arena[node_id].get().state.whose_turn();
                let stored = if parent_player == child_player { value } else { 1.0 - value };
                arena[node_id].get_mut().initial_value = Some(stored);
            }
            value
        };

        // Back-propagation - update the node and its ancestors with the evaluation result
        back_propagate(node_id, &mut arena, value);
    }

    // If there are no responses to the root state then return None
    if root_id.children(&arena).count() == 0 {
        return None;
    }

    // Get the best child (of root) by the number of visits, or None if there are no children
    let best_child_id = root_id.children(&arena).max_by(|&a, &b| {
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
fn select<G, E>(node_id: NodeId, arena: &Arena<Node<G::State>>, context: &Context<'_, G, E>) -> NodeId
where
    G: ResponseGenerator,
    E: ValueEstimator<State = G::State>,
{
    let c = context.c;
    let w = context.initial_value_weight;
    let mut selected = node_id;

    // Traverse the tree until a selectable node is found
    // If a node is not fully expanded, then select it for expansion.
    // If a node is terminal or has no children, then select it for evaluation.
    // Otherwise, descend to the child with the highest UCT value and continue.
    while !selectable(selected, arena) {
        let children: Vec<NodeId> = selected.children(arena).collect();
        let best_child = *children
            .iter()
            .max_by(|&a, &b| {
                let a_uct = arena.get(*a).unwrap().get().uct(*a, arena, c, w);
                let b_uct = arena.get(*b).unwrap().get().uct(*b, arena, c, w);
                a_uct.total_cmp(&b_uct)
            })
            .unwrap(); // Safe to unwrap because not_selectable ensures there are children
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
fn expand<G, E>(node_id: NodeId, arena: &mut Arena<Node<G::State>>, context: &Context<'_, G, E>) -> Option<NodeId>
where
    G: ResponseGenerator,
    E: ValueEstimator<State = G::State>,
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

// Eagerly expands ALL untried actions of a node, estimating each new child exactly once
//
// Each child stores its perspective-adjusted initial value ("wins for the player who
// just moved") and keeps zero visits. The best new child is chosen by argmax of the
// stored initial value via total_cmp - equivalent to the UCT ordering among the
// all-unvisited, equal-n_eff siblings, and well-defined even when the expanded node
// itself has zero visits (where UCT's parent-visits precondition fails).
//
// # Arguments
// * `node_id` - The node to expand
// * `arena` - The arena containing all nodes
// * `context` - The search context containing the response generator and estimator
//
// # Returns
// Some((chosen_child_id, chosen_child_raw_estimate)) if at least one action was
// untried, None otherwise. The raw estimate is from the chosen child state's
// current player's perspective, ready for back-propagation with the child as leaf.
fn expand_eager<G, E>(node_id: NodeId, arena: &mut Arena<Node<G::State>>, context: &Context<'_, G, E>) -> Option<(NodeId, f32)>
where
    G: ResponseGenerator,
    E: ValueEstimator<State = G::State, ResponseGenerator = G>,
{
    let untried = std::mem::take(&mut arena[node_id].get_mut().untried_actions);
    if untried.is_empty() {
        return None;
    }

    let parent_player = arena[node_id].get().state.whose_turn();
    let mut new_children: Vec<(NodeId, f32)> = Vec::with_capacity(untried.len());
    for action in untried {
        let child_state = arena[node_id].get().state.apply(&action);
        let raw = context.estimator.estimate(&child_state, context.response_generator);
        let stored = if parent_player == child_state.whose_turn() {
            raw
        } else {
            1.0 - raw
        };
        let mut child_node = Node::new(child_state, Some(action), context.response_generator);
        child_node.initial_value = Some(stored);
        let child_id = arena.new_node(child_node);
        node_id.append(child_id, arena);
        new_children.push((child_id, raw));
    }

    // Choose the best new child by its stored (perspective-adjusted) initial value
    new_children.into_iter().max_by(|a, b| {
        let a_v0 = arena[a.0].get().initial_value.unwrap_or(0.0);
        let b_v0 = arena[b.0].get().initial_value.unwrap_or(0.0);
        a_v0.total_cmp(&b_v0)
    })
}

// Evaluates the given node using the ValueEstimator implementation
//
// Returns the estimator's [0.0, 1.0] value for the node's state from the perspective of that
// state's current player.
//
// # Arguments
// * `node_id` - The node to evaluate
// * `arena` - The arena containing all nodes
// * `context` - The search context containing the value estimator implementation
//
// # Returns
// [0.0, 1.0] as the evaluation score for the node's game state, from the perspective of that
// state's current player
fn estimate_leaf<G, E>(node_id: NodeId, arena: &Arena<Node<G::State>>, context: &Context<'_, G, E>) -> f32
where
    G: ResponseGenerator,
    E: ValueEstimator<State = G::State, ResponseGenerator = G>,
{
    context
        .estimator
        .estimate(&arena[node_id].get().state, context.response_generator)
}

// Back-propagates the value up the tree
//
// Updates the visit count and value sum for the given node and all of its ancestors up to the
// root. `value` is in [0.0, 1.0] from the perspective of `whose_turn()` at the leaf node's state.
//
// A node's `value_sum` is stored from the perspective of the player who chose the action leading
// into it - i.e. `whose_turn()` of its parent node's state (the root has no parent, so its own
// `whose_turn()` is used instead; the root's value_sum is never consulted by selection). Each
// ancestor is credited `value` if its perspective player is the same as the leaf's current
// player, or `1.0 - value` otherwise. Perspective comparisons use `whose_turn()` equality, never
// ply parity, since a player may move twice in a row in some games.
//
// # Arguments
// * `leaf_id` - The leaf node whose evaluation is being back-propagated
// * `arena` - The arena containing all nodes
// * `value` - The value to propagate up the tree, from the perspective of the leaf's current player
fn back_propagate<S>(leaf_id: NodeId, arena: &mut Arena<Node<S>>, value: f32)
where
    S: State,
{
    let leaf_player = arena[leaf_id].get().state.whose_turn();
    let mut current = Some(leaf_id);
    while let Some(id) = current {
        let parent = arena[id].parent();
        // Perspective of the player who chose the action leading into this node;
        // the root uses its own state's player.
        let perspective = match parent {
            Some(parent_id) => arena[parent_id].get().state.whose_turn(),
            None => arena[id].get().state.whose_turn(),
        };
        let credit = if perspective == leaf_player { value } else { 1.0 - value };
        let node = arena[id].get_mut();
        node.visits += 1;
        node.value_sum += credit;
        current = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

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

        fn whose_turn(&self) -> PlayerId {
            PlayerId::Alice
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
            if state.terminal { vec![] } else { vec![TestAction::new(1)] }
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

    struct TestEstimator;

    impl ValueEstimator for TestEstimator {
        type State = TestGameState;
        type ResponseGenerator = TestResponseGenerator;

        fn estimate(&self, _state: &TestGameState, _rg: &TestResponseGenerator) -> f32 {
            0.5 // Simple fixed estimate for testing
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
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;

        // Test with minimal iterations
        let result = search(&state, &generator, &estimator, 1.0, 0.0, false, 1);
        // Since we have actions available, should return Some action
        assert!(result.is_some());
    }

    #[test]
    fn test_mcts_search_terminal_state() {
        let state = TestGameState {
            value: 0,
            terminal: true,
        };
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;

        // Terminal state should return None (no actions)
        let result = search(&state, &generator, &estimator, 1.0, 0.0, false, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_mcts_search_zero_iterations() {
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;

        // Zero iterations should still work
        let result = search(&state, &generator, &estimator, 1.0, 0.0, false, 0);
        // Might return None or Some depending on implementation
        // Just verify it doesn't crash
        let _ = result;
    }

    #[test]
    fn test_mcts_search_different_c_values() {
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;

        // Test with different exploration constants
        let result1 = search(&state, &generator, &estimator, 0.1, 0.0, false, 5);
        let result2 = search(&state, &generator, &estimator, 2.0, 0.0, false, 5);

        // Both should work (might return different results)
        assert!(result1.is_some() || state.terminal);
        assert!(result2.is_some() || state.terminal);
    }

    #[test]
    fn test_mcts_search_multiple_iterations() {
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;

        // Test with multiple iterations
        let result = search(&state, &generator, &estimator, 1.4, 0.0, false, 50);

        // Should return an action if state is not terminal
        if !state.terminal {
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_mcts_search_consistency() {
        let state = TestGameState {
            value: 5,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;

        // Multiple searches on same state should work
        let result1 = search(&state, &generator, &estimator, 1.0, 0.0, false, 10);
        let result2 = search(&state, &generator, &estimator, 1.0, 0.0, false, 10);

        // Both should return results (might be different due to randomness)
        assert!(result1.is_some());
        assert!(result2.is_some());
    }

    // Two-ply adversarial fixture: state ids form a fixed tree.
    //   0 (Alice) -a0-> 1 (Bob) -a0-> 3 terminal, Alice-value 0.9
    //                   1       -a1-> 4 terminal, Alice-value 0.1
    //   0 (Alice) -a1-> 2 (Bob) -a0-> 5 terminal, Alice-value 0.5
    //                   2       -a1-> 6 terminal, Alice-value 0.6
    // Bob minimizes Alice's value: line a0 yields 0.1 for Alice, line a1 yields 0.5.
    // Correct root choice is a1. Max-max selection chases the 0.9 leaf and picks a0.
    #[derive(Debug, Clone, PartialEq)]
    struct TwoPlyState {
        id: u8,
    }

    impl State for TwoPlyState {
        type Action = TwoPlyAction;

        fn fingerprint(&self) -> u64 {
            self.id as u64
        }

        fn whose_turn(&self) -> PlayerId {
            match self.id {
                1 | 2 => PlayerId::Bob,
                _ => PlayerId::Alice,
            }
        }

        fn is_terminal(&self) -> bool {
            self.id >= 3
        }

        fn apply(&self, action: &TwoPlyAction) -> Self {
            let id = match (self.id, action.id) {
                (0, 0) => 1,
                (0, 1) => 2,
                (1, 0) => 3,
                (1, 1) => 4,
                (2, 0) => 5,
                (2, 1) => 6,
                _ => panic!("illegal action"),
            };
            Self { id }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TwoPlyAction {
        id: u8,
    }

    struct TwoPlyResponseGenerator;

    impl ResponseGenerator for TwoPlyResponseGenerator {
        type State = TwoPlyState;

        fn generate(&self, state: &TwoPlyState) -> Vec<TwoPlyAction> {
            if state.is_terminal() {
                vec![]
            } else {
                vec![TwoPlyAction { id: 0 }, TwoPlyAction { id: 1 }]
            }
        }
    }

    struct TwoPlyEstimator;

    impl ValueEstimator for TwoPlyEstimator {
        type State = TwoPlyState;
        type ResponseGenerator = TwoPlyResponseGenerator;

        // [0,1] from the perspective of state.whose_turn(). Terminal states (ids 3-6)
        // have Alice to move, so their exact Alice-values are returned as-is.
        fn estimate(&self, state: &TwoPlyState, _rg: &TwoPlyResponseGenerator) -> f32 {
            match state.id {
                3 => 0.9,
                4 => 0.1,
                5 => 0.5,
                6 => 0.6,
                _ => 0.5, // non-terminal: neutral heuristic
            }
        }
    }

    #[test]
    fn test_mcts_adversarial_bob_minimizes() {
        let root = TwoPlyState { id: 0 };
        let rg = TwoPlyResponseGenerator;
        let estimator = TwoPlyEstimator;
        let result = search(&root, &rg, &estimator, DEFAULT_EXPLORATION_CONSTANT, 0.0, false, 200);
        assert_eq!(result, Some(TwoPlyAction { id: 1 }));
    }

    // Builds a bare node for direct UCT testing (state contents are irrelevant)
    fn test_node(visits: u32, value_sum: f32, iv0: Option<f32>) -> Node<TestGameState> {
        Node {
            state: TestGameState {
                value: 0,
                terminal: false,
            },
            action: None,
            untried_actions: vec![],
            visits,
            value_sum,
            initial_value: iv0,
        }
    }

    struct CountingEstimator {
        calls: Cell<u32>,
    }

    impl ValueEstimator for CountingEstimator {
        type State = TestGameState;
        type ResponseGenerator = TestResponseGenerator;

        fn estimate(&self, _state: &TestGameState, _rg: &TestResponseGenerator) -> f32 {
            self.calls.set(self.calls.get() + 1);
            0.5
        }
    }

    #[test]
    fn test_uct_initial_value_blend() {
        let mut arena: Arena<Node<TestGameState>> = Arena::new();
        let parent = arena.new_node(test_node(4, 0.0, None));
        let unvisited = arena.new_node(test_node(0, 0.0, Some(0.8)));
        parent.append(unvisited, &mut arena);
        let visited = arena.new_node(test_node(2, 0.5, Some(0.8)));
        parent.append(visited, &mut arena);
        let c = 1.0;

        // visits=0, v0=0.8, w=2: n_eff=2, Q=v0=0.8, UCT=0.8+sqrt(ln4/2)
        let u = arena[unvisited].get().uct(unvisited, &arena, c, 2.0);
        assert!((u - 1.632_554_6).abs() < 1e-4, "got {u}");

        // visits=2, sum=0.5, v0=0.8, w=2: n_eff=4, Q=(0.5+1.6)/4=0.525, UCT=0.525+sqrt(ln4/4)
        let u2 = arena[visited].get().uct(visited, &arena, c, 2.0);
        assert!((u2 - 1.113_705).abs() < 1e-4, "got {u2}");

        // Larger w pulls Q toward v0: w=6: n_eff=8, Q=(0.5+4.8)/8=0.6625, UCT=0.6625+sqrt(ln4/8)
        let u3 = arena[visited].get().uct(visited, &arena, c, 6.0);
        assert!((u3 - 1.078_777_3).abs() < 1e-4, "got {u3}");

        // Q decays from pure v0 toward the observed mean as real visits accumulate:
        // observed mean 0.25 < blended Q(w=2) 0.525 < v0 0.8
        let q_w2 = 0.525_f32;
        assert!(q_w2 > 0.25 && q_w2 < 0.8);
    }

    #[test]
    fn test_uct_zero_weight_matches_legacy() {
        let mut arena: Arena<Node<TestGameState>> = Arena::new();
        let parent = arena.new_node(test_node(10, 0.0, None));
        let child = arena.new_node(test_node(3, 1.2, Some(0.9)));
        parent.append(child, &mut arena);
        let c = std::f32::consts::SQRT_2;

        // w = 0 must reproduce the legacy formula bit-for-bit even when an
        // initial value is present
        let expected = 1.2_f32 / 3.0 + c * ((10_f32).ln() / 3.0).sqrt();
        assert_eq!(arena[child].get().uct(child, &arena, c, 0.0), expected);

        // visits == 0 with w == 0 falls back to INFINITY even with an estimate
        let unvisited = arena.new_node(test_node(0, 0.0, Some(0.9)));
        parent.append(unvisited, &mut arena);
        assert_eq!(arena[unvisited].get().uct(unvisited, &arena, c, 0.0), f32::INFINITY);

        // visits == 0 with no estimate is INFINITY even with w > 0
        let no_estimate = arena.new_node(test_node(0, 0.0, None));
        parent.append(no_estimate, &mut arena);
        assert_eq!(arena[no_estimate].get().uct(no_estimate, &arena, c, 2.0), f32::INFINITY);
    }

    #[test]
    fn test_lazy_estimator_call_count() {
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = CountingEstimator { calls: Cell::new(0) };

        // Lazy mode runs the estimator exactly once per iteration
        let result = search(&state, &generator, &estimator, 1.0, 0.0, false, 25);
        assert!(result.is_some());
        assert_eq!(estimator.calls.get(), 25);
    }

    #[test]
    fn test_eager_estimator_call_count() {
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = TestResponseGenerator;
        let estimator = CountingEstimator { calls: Cell::new(0) };

        // Within 5 iterations no terminal state is reachable, so every iteration
        // selects a childless node with 2 untried actions: eager mode makes exactly
        // one estimator call per created child = 2 per iteration.
        let result = search(&state, &generator, &estimator, 1.0, 0.5, true, 5);
        assert!(result.is_some());
        assert_eq!(estimator.calls.get(), 10);
    }

    #[test]
    fn test_eager_expansion_invariants() {
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;
        let context = Context {
            response_generator: &generator,
            estimator: &estimator,
            c: 1.0,
            initial_value_weight: 0.5,
            estimate_on_expansion: true,
        };
        let mut arena: Arena<Node<TestGameState>> = Arena::new();
        let root_state = TestGameState {
            value: 0,
            terminal: false,
        };
        let root_id = arena.new_node(Node::new(root_state, None, &generator));
        arena.get_mut(root_id).unwrap().get_mut().visits = 1;

        let (chosen_id, raw) = expand_eager(root_id, &mut arena, &context).unwrap();
        back_propagate(chosen_id, &mut arena, raw);

        // Parent is fully expanded after ONE eager expansion
        assert!(arena[root_id].get().fully_expanded());
        let children: Vec<NodeId> = root_id.children(&arena).collect();
        assert_eq!(children.len(), 2);
        // Every child has a stored initial value
        for &child in &children {
            assert!(arena[child].get().initial_value.is_some());
        }
        // Exactly one child (the chosen one) was visited; siblings were not
        // back-propagated
        assert_eq!(children.iter().filter(|&&id| arena[id].get().visits == 1).count(), 1);
        assert_eq!(children.iter().filter(|&&id| arena[id].get().visits == 0).count(), 1);
        assert_eq!(arena[chosen_id].get().visits, 1);
    }

    // Direct unit test of back_propagate's perspective flip, isolated from a full search.
    // root(Alice, id0) -> mid(Bob, id1) -> leaf(Alice, id3). A node's stats are stored from
    // the perspective of whoever chose the move into it: the leaf's stats reflect Bob's
    // choice (the mid->leaf hop), which disagrees with the leaf's own whose_turn() (Alice),
    // so the leaf is credited 1-value. Both mid (Alice's choice at the root) and the root
    // itself agree with the leaf's player (Alice), so they're credited `value` unchanged.
    #[test]
    fn test_back_propagate_perspective_three_ply() {
        let rg = TwoPlyResponseGenerator;
        let mut arena: Arena<Node<TwoPlyState>> = Arena::new();

        let root_id = arena.new_node(Node::new(TwoPlyState { id: 0 }, None, &rg));
        let mid_id = arena.new_node(Node::new(TwoPlyState { id: 1 }, Some(TwoPlyAction { id: 0 }), &rg));
        root_id.append(mid_id, &mut arena);
        let leaf_id = arena.new_node(Node::new(TwoPlyState { id: 3 }, Some(TwoPlyAction { id: 0 }), &rg));
        mid_id.append(leaf_id, &mut arena);

        let value = 0.9_f32;
        back_propagate(leaf_id, &mut arena, value);

        assert_eq!(arena[leaf_id].get().visits, 1);
        assert!((arena[leaf_id].get().value_sum - (1.0 - value)).abs() < 1e-6);

        assert_eq!(arena[mid_id].get().visits, 1);
        assert!((arena[mid_id].get().value_sum - value).abs() < 1e-6);

        assert_eq!(arena[root_id].get().visits, 1);
        assert!((arena[root_id].get().value_sum - value).abs() < 1e-6);
    }

    // Visit-conservation invariants: every iteration walks exactly one root-to-leaf path,
    // so after `iterations` iterations the root (visited once up front, then once per
    // iteration) must have `1 + iterations` visits, and that same iteration count must be
    // exactly distributed across the root's immediate children.
    #[test]
    fn test_mcts_visit_conservation_lazy() {
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;
        let context = Context {
            response_generator: &generator,
            estimator: &estimator,
            c: 1.0,
            initial_value_weight: 0.0,
            estimate_on_expansion: false,
        };
        let mut arena: Arena<Node<TestGameState>> = Arena::new();
        let root_state = TestGameState {
            value: 0,
            terminal: false,
        };
        let root_id = arena.new_node(Node::new(root_state, None, &generator));
        arena.get_mut(root_id).unwrap().get_mut().visits = 1;

        let iterations = 30;
        for _ in 0..iterations {
            let mut node_id = select(root_id, &arena, &context);
            let expanded = expand(node_id, &mut arena, &context);
            if let Some(child_id) = expanded {
                node_id = child_id;
            }
            let value = estimate_leaf(node_id, &arena, &context);
            if expanded.is_some() {
                let parent_id = arena[node_id].parent().expect("expanded child has a parent");
                let parent_player = arena[parent_id].get().state.whose_turn();
                let child_player = arena[node_id].get().state.whose_turn();
                let stored = if parent_player == child_player { value } else { 1.0 - value };
                arena[node_id].get_mut().initial_value = Some(stored);
            }
            back_propagate(node_id, &mut arena, value);
        }

        assert_eq!(arena[root_id].get().visits, 1 + iterations);
        let children_visits: u32 = root_id.children(&arena).map(|c| arena[c].get().visits).sum();
        assert_eq!(children_visits, iterations);
    }

    #[test]
    fn test_mcts_visit_conservation_eager() {
        let generator = TestResponseGenerator;
        let estimator = TestEstimator;
        let context = Context {
            response_generator: &generator,
            estimator: &estimator,
            c: 1.0,
            initial_value_weight: 0.0,
            estimate_on_expansion: true,
        };
        let mut arena: Arena<Node<TestGameState>> = Arena::new();
        let root_state = TestGameState {
            value: 0,
            terminal: false,
        };
        let root_id = arena.new_node(Node::new(root_state, None, &generator));
        arena.get_mut(root_id).unwrap().get_mut().visits = 1;

        let iterations = 30;
        for _ in 0..iterations {
            let node_id = select(root_id, &arena, &context);
            let (leaf_id, value) = if let Some((child_id, raw)) = expand_eager(node_id, &mut arena, &context) {
                (child_id, raw)
            } else {
                (node_id, estimate_leaf(node_id, &arena, &context))
            };
            back_propagate(leaf_id, &mut arena, value);
        }

        assert_eq!(arena[root_id].get().visits, 1 + iterations);
        let children_visits: u32 = root_id.children(&arena).map(|c| arena[c].get().visits).sum();
        assert_eq!(children_visits, iterations);
    }

    struct SingleEstimator;

    impl ValueEstimator for SingleEstimator {
        type State = TestGameState;
        type ResponseGenerator = SingleResponseGenerator;

        fn estimate(&self, _state: &TestGameState, _rg: &SingleResponseGenerator) -> f32 {
            0.5
        }
    }

    // search()-level sanity check: with only one legal action available, it must be the
    // one returned, regardless of the estimator or exploration constant.
    #[test]
    fn test_mcts_search_single_action_is_returned() {
        let state = TestGameState {
            value: 0,
            terminal: false,
        };
        let generator = SingleResponseGenerator;
        let estimator = SingleEstimator;

        let result = search(&state, &generator, &estimator, DEFAULT_EXPLORATION_CONSTANT, 0.0, false, 5);
        assert_eq!(result.map(|a| a.increment), Some(1));
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ImmediateState {
        id: u8,
    }

    impl State for ImmediateState {
        type Action = ImmediateAction;

        fn fingerprint(&self) -> u64 {
            self.id as u64
        }

        fn whose_turn(&self) -> PlayerId {
            PlayerId::Alice
        }

        fn is_terminal(&self) -> bool {
            self.id != 0
        }

        fn apply(&self, action: &ImmediateAction) -> Self {
            Self { id: action.id }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ImmediateAction {
        id: u8,
    }

    struct ImmediateResponseGenerator;

    impl ResponseGenerator for ImmediateResponseGenerator {
        type State = ImmediateState;

        fn generate(&self, state: &ImmediateState) -> Vec<ImmediateAction> {
            if state.is_terminal() {
                vec![]
            } else {
                vec![ImmediateAction { id: 1 }, ImmediateAction { id: 2 }]
            }
        }
    }

    struct ImmediateEstimator;

    impl ValueEstimator for ImmediateEstimator {
        type State = ImmediateState;
        type ResponseGenerator = ImmediateResponseGenerator;

        // Terminal states return their exact outcome: id 1 is a win, id 2 a loss.
        fn estimate(&self, state: &ImmediateState, _rg: &ImmediateResponseGenerator) -> f32 {
            match state.id {
                1 => 1.0,
                2 => 0.0,
                _ => 0.5,
            }
        }
    }

    // Basic value-sensitivity check: given a choice between an immediate win and an
    // immediate loss, the search must prefer the win well before its iteration budget
    // is exhausted.
    #[test]
    fn test_mcts_prefers_immediate_win_over_loss() {
        let root = ImmediateState { id: 0 };
        let rg = ImmediateResponseGenerator;
        let estimator = ImmediateEstimator;
        let result = search(&root, &rg, &estimator, DEFAULT_EXPLORATION_CONSTANT, 0.0, false, 30);
        assert_eq!(result, Some(ImmediateAction { id: 1 }));
    }

    #[derive(Debug, Clone, PartialEq)]
    struct DoubleMoveState {
        id: u8,
    }

    impl State for DoubleMoveState {
        type Action = DoubleMoveAction;

        fn fingerprint(&self) -> u64 {
            self.id as u64
        }

        // Alice moves on every ply: this is the "double move" case the module docs call
        // out, where perspective must track whose_turn() equality rather than ply parity.
        fn whose_turn(&self) -> PlayerId {
            PlayerId::Alice
        }

        fn is_terminal(&self) -> bool {
            self.id >= 3
        }

        fn apply(&self, action: &DoubleMoveAction) -> Self {
            let id = match (self.id, action.id) {
                (0, 0) => 1,
                (0, 1) => 2,
                (1, 0) => 3,
                (1, 1) => 4,
                (2, 0) => 5,
                (2, 1) => 6,
                _ => panic!("illegal action"),
            };
            Self { id }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct DoubleMoveAction {
        id: u8,
    }

    struct DoubleMoveResponseGenerator;

    impl ResponseGenerator for DoubleMoveResponseGenerator {
        type State = DoubleMoveState;

        fn generate(&self, state: &DoubleMoveState) -> Vec<DoubleMoveAction> {
            if state.is_terminal() {
                vec![]
            } else {
                vec![DoubleMoveAction { id: 0 }, DoubleMoveAction { id: 1 }]
            }
        }
    }

    struct DoubleMoveEstimator;

    impl ValueEstimator for DoubleMoveEstimator {
        type State = DoubleMoveState;
        type ResponseGenerator = DoubleMoveResponseGenerator;

        fn estimate(&self, state: &DoubleMoveState, _rg: &DoubleMoveResponseGenerator) -> f32 {
            match state.id {
                3 => 0.9,
                4 => 0.2,
                5 => 0.3,
                6 => 0.6,
                _ => 0.5,
            }
        }
    }

    // Alice moves twice in a row (id0 -> id1/id2 -> terminal) with no adversary at any
    // level. Correct play is pure maximization along both plies: from id1 the best line
    // reaches 0.9, from id2 the best line reaches 0.6, so the root must prefer id1 (a0).
    // A perspective scheme keyed on ply parity instead of whose_turn() equality would
    // wrongly flip credit between these two same-player hops and could corrupt this
    // choice; this reproduces the exact scenario module docs warn about ("a player may
    // move twice in a row in some games").
    #[test]
    fn test_mcts_double_move_same_player() {
        let root = DoubleMoveState { id: 0 };
        let rg = DoubleMoveResponseGenerator;
        let estimator = DoubleMoveEstimator;
        let result = search(&root, &rg, &estimator, DEFAULT_EXPLORATION_CONSTANT, 0.0, false, 500);
        assert_eq!(result, Some(DoubleMoveAction { id: 0 }));
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ThreePlyState {
        id: u8,
    }

    impl State for ThreePlyState {
        type Action = ThreePlyAction;

        fn fingerprint(&self) -> u64 {
            self.id as u64
        }

        fn whose_turn(&self) -> PlayerId {
            match self.id {
                1 | 2 => PlayerId::Bob,
                _ => PlayerId::Alice,
            }
        }

        fn is_terminal(&self) -> bool {
            self.id >= 7
        }

        fn apply(&self, action: &ThreePlyAction) -> Self {
            let id = match (self.id, action.id) {
                (0, 0) => 1,
                (0, 1) => 2,
                (1, 0) => 3,
                (1, 1) => 4,
                (2, 0) => 5,
                (2, 1) => 6,
                (3, 0) => 7,
                (3, 1) => 8,
                (4, 0) => 9,
                (4, 1) => 10,
                (5, 0) => 11,
                (5, 1) => 12,
                (6, 0) => 13,
                (6, 1) => 14,
                _ => panic!("illegal action"),
            };
            Self { id }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ThreePlyAction {
        id: u8,
    }

    struct ThreePlyResponseGenerator;

    impl ResponseGenerator for ThreePlyResponseGenerator {
        type State = ThreePlyState;

        fn generate(&self, state: &ThreePlyState) -> Vec<ThreePlyAction> {
            if state.is_terminal() {
                vec![]
            } else {
                vec![ThreePlyAction { id: 0 }, ThreePlyAction { id: 1 }]
            }
        }
    }

    struct ThreePlyEstimator;

    impl ValueEstimator for ThreePlyEstimator {
        type State = ThreePlyState;
        type ResponseGenerator = ThreePlyResponseGenerator;

        fn estimate(&self, state: &ThreePlyState, _rg: &ThreePlyResponseGenerator) -> f32 {
            match state.id {
                7 => 0.9,
                8 => 0.2,
                9 => 0.1,
                10 => 0.3,
                11 => 0.6,
                12 => 0.4,
                13 => 0.5,
                14 => 0.7,
                _ => 0.5,
            }
        }
    }

    // Alice-Bob-Alice, 8 terminal leaves:
    //   id1 (Bob) chooses between id3 (Alice's best reachable = max(0.9, 0.2) = 0.9)
    //             and id4 (Alice's best reachable = max(0.1, 0.3) = 0.3) -> minimizes to 0.3
    //   id2 (Bob) chooses between id5 (Alice's best reachable = max(0.6, 0.4) = 0.6)
    //             and id6 (Alice's best reachable = max(0.5, 0.7) = 0.7) -> minimizes to 0.6
    // Alice maximizes at the root: 0.6 (via id2, action a1) > 0.3 (via id1, action a0).
    // A max-max search that ignored Bob's minimization would instead chase the 0.9 leaf
    // under id1 and wrongly pick a0. This extends the module's 2-ply adversarial check to
    // a deeper tree with two independent Bob-minimizing subtrees.
    #[test]
    fn test_mcts_three_ply_adversarial_minimax() {
        let root = ThreePlyState { id: 0 };
        let rg = ThreePlyResponseGenerator;
        let estimator = ThreePlyEstimator;
        let result = search(&root, &rg, &estimator, DEFAULT_EXPLORATION_CONSTANT, 0.0, false, 3000);
        assert_eq!(result, Some(ThreePlyAction { id: 1 }));
    }

    // The 2-ply adversarial fixture's correct answer (a1) must hold regardless of
    // expansion strategy (lazy/eager) or initial-value weighting.
    #[test]
    fn test_mcts_adversarial_correctness_across_modes() {
        let rg = TwoPlyResponseGenerator;
        let estimator = TwoPlyEstimator;
        for &(eager, w) in &[(false, 0.0_f32), (true, 0.0), (true, 1.0), (false, 1.0)] {
            let root = TwoPlyState { id: 0 };
            let result = search(&root, &rg, &estimator, DEFAULT_EXPLORATION_CONSTANT, w, eager, 200);
            assert_eq!(
                result,
                Some(TwoPlyAction { id: 1 }),
                "failed for estimate_on_expansion={eager}, initial_value_weight={w}"
            );
        }
    }
}
