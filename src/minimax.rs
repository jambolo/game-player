//! Minimax Game Tree Search Implementation
//!
//! This module implements a game tree search using min-max strategy, alpha-beta pruning, and a transposition table. The
//! game-specific components are provided by the user using the traits defined in other modules.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use crate::minimax::{search, ResponseGenerator};
//! use crate::transposition_table::TranspositionTable;
//!
//! // Assuming you have implemented the required traits for your game
//! let tt = Rc::new(RefCell::new(TranspositionTable::new(1000, 100)));
//! let static_evaluator = MyStaticEvaluator::new();
//! let response_generator = MyResponseGenerator::new();
//! let initial_state = Rc::new(MyGameState::new());
//!
//! if let Some(best_move) = search(&tt, &static_evaluator, &response_generator, &initial_state, 6) {
//!     println!("Best move found: {:?}", best_move);
//! }
//! ```
//!
//! # Notes
//! - The search assumes a two-player zero-sum game with perfect information.
//! - The transposition table can be reused across multiple searches for efficiency and support of iterative deepening.

use std::cell::RefCell;
use std::rc::Rc;

use crate::state::*;
use crate::static_evaluator::*;
use crate::transposition_table::*;

static SEF_QUALITY: i16 = 0; // Quality of a value returned by the static evaluation function

// Holds evaluation information about a response.
struct Response<S> {
    // Reference to the resulting state
    state: Rc<S>,
    // Value of the state
    value: f32,
    // Quality of the value. Quality is the number of plies searched to find the value.
    quality: i16,
}

// Holds static information pertaining to the search.
struct Context<'a, S, E: StaticEvaluator<S>, R: ResponseGenerator<State = S>>
where
    S: State,
{
    max_depth: i32,
    rg: &'a R,
    sef: &'a E,
    tt: &'a Rc<RefCell<TranspositionTable>>,
    _phantom: std::marker::PhantomData<S>,
}
/// Response generator function object trait.
///
/// This trait defines the interface for generating all possible responses from a given state. Implementers should provide
/// game-specific logic for move generation.
///
/// # Examples
/// ```rust,ignore
/// use std::rc::Rc;
/// use crate::minimax::ResponseGenerator;
///
/// struct MyResponseGenerator;
///
/// impl ResponseGenerator for MyResponseGenerator {
///     type State = MyGameState;
///
///     fn generate(&self, state: &Rc<Self::State>, depth: i32) -> Vec<Box<Self::State>> {
///         // Generate all valid moves for the current player
///         let mut responses = Vec::new();
///
///         // Game-specific logic to generate moves
///         for possible_move in get_all_valid_moves(state) {
///             let new_state = state.apply_move(possible_move);
///             responses.push(Box::new(new_state));
///         }
///
///         responses
///     }
/// }
/// ```
///
/// # Implementation Notes
/// - Return an empty vector if no moves are available (player cannot respond)
/// - If passing is allowed in the game, include a "pass" move as a valid response
/// - The depth parameter can be used for depth-dependent move generation optimizations
pub trait ResponseGenerator {
    /// The type representing game states that this generator works with
    type State: State;

    /// Generates a list of all possible responses to the given state.
    ///
    /// This method should return all legal moves available to the current player in the given state. The implementation
    /// should be game-specific and handle all rules and constraints of the particular game being played.
    ///
    /// # Arguments
    /// * `state` - The current state to generate responses for
    /// * `depth` - Current search depth (ply number), useful for optimizations
    ///
    /// # Returns
    /// A vector of boxed game states representing all possible moves.
    /// Returns an empty vector if no moves are available.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let response_gen = MyResponseGenerator::new();
    /// let current_state = Rc::new(MyGameState::new());
    ///
    /// let possible_moves = response_gen.generate(&current_state, 0);
    /// println!("Found {} possible moves", possible_moves.len());
    /// ```
    ///
    /// # Note
    /// The caller gains ownership of the returned states.
    ///
    /// # Note
    /// Returning no responses indicates that the player cannot respond. It does not necessarily indicate that the game is
    /// over or that the player has passed. If passing is allowed, then a "pass" state should be a valid response.
    fn generate(&self, state: &Rc<Self::State>, depth: i32) -> Vec<Box<Self::State>>;
}

/// A minimax search implementation using alpha-beta pruning and a transposition table.
///
/// This function performs a complete minimax search to find the best move for the current player. It uses alpha-beta pruning for
/// efficiency and a transposition table to avoid redundant calculations.
///
/// # Type Parameters
/// * `S` - Game state type that implements the `State<A>` trait
/// * `E` - Static evaluator type that implements `StaticEvaluator<S>`
/// * `A` - Action/move type used by the game
/// * `R` - Response generator type that implements `ResponseGenerator<S>`
///
/// # Arguments
/// * `tt` - A transposition table for caching previously computed positions. Can be reused across multiple searches.
/// * `sef` - The static evaluation function
/// * `rg` - The response generator
/// * `s0` - The state to search from
/// * `max_depth` - Maximum search depth in plies
///
/// # Returns
/// `Some(Rc<S>)` containing the best move found, or `None` if no valid moves exist
///
/// # Examples
///
/// ```rust,ignore
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use crate::minimax::search;
/// use crate::transposition_table::TranspositionTable;
///
/// // Set up the search components
/// let transposition_table = Rc::new(RefCell::new(TranspositionTable::new(1000, 100)));
/// let evaluator = MyStaticEvaluator::new();
/// let move_generator = MyResponseGenerator::new();
/// let game_state = Rc::new(MyGameState::initial_position());
///
/// // Search to depth 6
/// match search(&transposition_table, &evaluator, &move_generator, &game_state, 6) {
///     Some(best_move) => println!("Best move: {:?}", best_move),
///     None => println!("No moves available"),
/// }
/// ```
///
/// # Algorithm Details
///
/// The search uses:
/// - **Minimax**: Alternating maximizing and minimizing players
/// - **Alpha-beta pruning**: Early termination of unpromising branches
/// - **Transposition table**: Caching of previously evaluated positions
/// - **Move ordering**: Better moves searched first for more effective pruning
pub fn search<S, E, R>(
    tt: &Rc<RefCell<TranspositionTable>>,
    sef: &E,
    rg: &R,
    s0: &Rc<S>,
    max_depth: i32,
) -> Option<Rc<S>>
where
    S: State,
    E: StaticEvaluator<S>,
    R: ResponseGenerator<State = S>,
{
    let context = Context {
        tt,
        sef,
        rg,
        max_depth,
        _phantom: std::marker::PhantomData,
    };
    if s0.whose_turn() == PlayerId::ALICE as u8 {
        if let Some(response) = alice_search(&context, s0, -f32::INFINITY, f32::INFINITY, 0) {
            return Some(Rc::clone(&response.state));
        }
    } else {
        if let Some(response) = bob_search(&context, s0, -f32::INFINITY, f32::INFINITY, 0) {
            return Some(Rc::clone(&response.state));
        }
    }
    None
}

// Evaluates all of Alice's possible responses to the given state. The returned response is the one with the highest value.
fn alice_search<S, E, R>(
    context: &Context<S, E, R>,
    state: &Rc<S>,
    mut alpha: f32,
    beta: f32,
    depth: i32,
) -> Option<Response<S>>
where
    S: State,
    E: StaticEvaluator<S>,
    R: ResponseGenerator<State = S>,
{
    // Depth of responses to this state
    let response_depth = depth + 1;
    // Quality of a response as a result of a search at this depth.
    let search_quality = (context.max_depth - response_depth) as i16;

    // Generate a list of the possible responses to this state by Alice. The responses are initialized with preliminary values.
    let mut responses = generate_responses(context, state, depth);

    // If there are no responses, return without a response. It's up to the caller to decide how to handle this case.
    if responses.is_empty() {
        return None;
    }

    // Sort from highest to lowest in order to increase the chance of triggering a beta cutoff earlier.
    responses.sort_by(|a, b| {
        b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Evaluate each of the responses and choose the one with the highest value
    let mut best_state: Option<&Rc<S>> = None;
    let mut best_value = -f32::INFINITY;
    let mut best_quality = -1;
    let mut pruned = false;

    for response in &responses {
        // Replace the preliminary value and quality of this response with the value and quality of Bob's subsequent response
        // to it.
        // The following conditions will cause the search to be skipped:
        // 1. The preliminary value indicates a win for Alice.
        // 2. The preliminary quality is more than the quality of a search. This can be a result of obtaining the preliminary
        //    value from the result of a previous search stored in the transposition table.
        // 3. The search has reached its maximum depth.
        let mut value = response.value;
        let mut quality = response.quality;
        if value < context.sef.alice_wins_value()
            && response_depth < context.max_depth
            && quality < search_quality
        {
            // Update the value of Alice's response by evaluating Bob's responses to it. If Bob has no response, then
            // leave the response's value and quality as is.
            if let Some(bob_response) =
                bob_search(&context, &response.state, alpha, beta, response_depth)
            {
                value = bob_response.value;
                quality = bob_response.quality;
            }
        }

        // Determine if this response's value is the best so far. If so, then save the value and do alpha-beta pruning
        if value > best_value {
            // Save it
            best_state = Some(&response.state);
            best_value = value;
            best_quality = quality;

            // If Alice wins with this response, then there is no reason to look for anything better
            if best_value >= context.sef.alice_wins_value() {
                break;
            }

            // alpha-beta pruning (beta cutoff) Here's how it works:
            //
            // Bob is looking for the lowest value. The 'beta' is the value of Bob's best response found so far in the previous
            // ply. If the value of this response is higher than the beta, then Bob will never choose a response leading to this
            // response because the result is worse than the result of a response Bob has already found. As such, there is no
            // reason to continue.
            if best_value > beta {
                // Beta cutoff
                pruned = true;
                break;
            }

            // alpha-beta pruning (alpha) Here's how it works:
            //
            // Alice is looking for the highest value. The 'alpha' is the value of Alice's best response found so far. If the
            // value of this response is higher than the alpha, then it is a better response for Alice. The alpha is
            // subsequently passed to Bob's search so that if Bob finds a response with a lower value than the alpha, then
            // there is no reason to continue because Alice already has a better response and will choose it instead of allowing
            // Bob to make a move with a lower value.
            if best_value > alpha {
                alpha = best_value;
            }
        }
    }

    assert!(best_value > -f32::INFINITY); // Sanity check
    assert!(best_quality >= 0); // Sanity check
    assert!(best_state.is_some()); // Sanity check

    // Just in case
    best_state.as_ref()?;

    // At this point, the value of this state becomes the value of the best response to it, and the quality becomes its
    // quality + 1.
    //
    // Save the value of this state in the T-table if the ply was not pruned. Pruning results in an incorrect value because the
    // search was interrupted and potentially better responses were not considered.
    if !pruned {
        context.tt.borrow_mut().update(state.fingerprint(), best_value, best_quality + 1);
    }

    Some(Response::<S> {
        state: Rc::clone(best_state?),
        value: best_value,
        quality: best_quality + 1,
    })
}

// Evaluates all of Bob's possible responses to the given state. The returned response is the one with the lowest value.
fn bob_search<S, E, R>(
    context: &Context<S, E, R>,
    state: &Rc<S>,
    alpha: f32,
    mut beta: f32,
    depth: i32,
) -> Option<Response<S>>
where
    S: State,
    E: StaticEvaluator<S>,
    R: ResponseGenerator<State = S>,
{
    // Depth of responses to this state
    let response_depth = depth + 1;
    // Quality of a response as a result of a search at this depth.
    let search_quality = (context.max_depth - response_depth) as i16;

    // Generate a list of the possible responses to this state by Bob. The responses are initialized with preliminary values.
    let mut responses = generate_responses(context, state, depth);

    // If there are no responses, return without a response. It's up to the caller to decide how to handle this case.
    if responses.is_empty() {
        return None;
    }

    // Sort from lowest to highest in order to increase the chance of triggering an alpha cutoff earlier
    responses.sort_by(|a, b| {
        a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Evaluate each of the responses and choose the one with the lowest value
    let mut best_state: Option<&Rc<S>> = None;
    let mut best_value = f32::INFINITY;
    let mut best_quality = -1;
    let mut pruned = false;

    for response in &responses {
        // Replace the preliminary value and quality of this response with the value and quality of Alice's subsequent response
        // to it.
        // The following conditions will cause the search to be skipped:
        // 1. The preliminary value indicates a win for Bob.
        // 2. The preliminary quality is more than the quality of a search. This can be a result of obtaining the preliminary
        //    value from the result of a previous search stored in the transposition table.
        // 3. The search has reached its maximum depth.
        let mut value = response.value;
        let mut quality = response.quality;
        if value > context.sef.bob_wins_value() && response_depth < context.max_depth && quality < search_quality
        {
            // Update the value of Bob's response by evaluating Alice's responses to it. If Alice has no response, then
            // leave the response's value and quality as is.
            if let Some(alice_response) =
                alice_search(context, &response.state, alpha, beta, response_depth)
            {
                value = alice_response.value;
                quality = alice_response.quality;
            }
        }

        // Determine if this response's value is the best so far. If so, then save the value and do alpha-beta pruning
        if value < best_value {
            // Save it
            best_state = Some(&response.state);
            best_value = value;
            best_quality = quality;

            // If Bob wins with this response, then there is no reason to look for anything better
            if best_value <= context.sef.bob_wins_value() {
                break;
            }

            // alpha-beta pruning (alpha cutoff) Here's how it works:
            //
            // Alice is looking for the highest value. The 'alpha' is the value of Alice's best move found so far in the previous
            // ply. If the value of this response is lower than the alpha, then Alice will never choose a response leading to this
            // response because the result is worse than the result of a response Alice has already found. As such, there is no
            // reason to continue.

            if best_value < alpha {
                // Alpha cutoff
                pruned = true;
                break;
            }

            // alpha-beta pruning (beta) Here's how it works:
            //
            // Bob is looking for the lowest value. The 'beta' is the value of Bob's best response found so far. If the
            // value of this response is lower than the beta, then it is a better response for Bob. The beta is subsequently passed
            // to Alice's search so that if Alice finds a response with a higher value than the beta, then there is no reason to
            // continue because Bob already has a better response and will choose it instead of allowing Alice to make a move with
            // a higher value.
            if best_value < beta {
                beta = best_value;
            }
        }
    }

    assert!(best_value < f32::INFINITY); // Sanity check
    assert!(best_quality >= 0); // Sanity check
    assert!(best_state.is_some()); // Sanity check

    // Just in case
    best_state.as_ref()?;

    // At this point, the value of this state becomes the value of the best response to it, and the quality becomes its
    // quality + 1.
    //
    // Save the value of this state in the T-table if the ply was not pruned. Pruning results in an incorrect value because the
    // search was interrupted and potentially better responses were not considered.
    if !pruned {
        context.tt.borrow_mut().update(state.fingerprint(), best_value, best_quality + 1);
    }

    Some(Response::<S> {
        state: Rc::clone(best_state?),
        value: best_value,
        quality: best_quality + 1,
    })
}

// Generates a list of responses to the given node
fn generate_responses<S, E, R>(
    context: &Context<S, E, R>,
    state: &Rc<S>,
    depth: i32,
) -> Vec<Response<S>>
where
    S: State,
    E: StaticEvaluator<S>,
    R: ResponseGenerator<State = S>,
{
    // Handle the case where node.state might be None
    let responses = context.rg.generate(state, depth);
    responses
        .into_iter()
        .map(|state| {
            let rc_state = Rc::from(state);
            let (value, quality) = get_preliminary_value(context, &rc_state);
            Response::<S> {
                state: rc_state,
                value,
                quality,
            }
        })
        .collect()
}

// Get a preliminary value of the state from the static evaluator or the transposition table
fn get_preliminary_value<S, E, R>(
    context: &Context<S, E, R>,
    state: &Rc<S>,
) -> (f32, i16)
where
    S: State,
    E: StaticEvaluator<S>,
    R: ResponseGenerator<State = S>,
{
    // SEF optimization:
    // Since any value of any state in the T-table has already been computed by search and/or SEF, it has a quality that is at
    // least as good as the quality of the value returned by the SEF. So, if the state being evaluated is in the T-table, then
    // the value in the T-table is used instead of running the SEF because T-table lookup is so much faster than the SEF.

    // If it is in the T-table then use that value, otherwise evaluate the state and save the value.
    let fingerprint = state.fingerprint();

    // First, check if the value is in the transposition table
    if let Some(cached_value) = context.tt.borrow_mut().check(fingerprint, -1) {
        return cached_value;
    }

    // Value not in table, so evaluate with static evaluator and store result
    let value = context.sef.evaluate(state);
    context
        .tt
        .borrow_mut()
        .update(fingerprint, value, SEF_QUALITY);
    (value, SEF_QUALITY)
}
