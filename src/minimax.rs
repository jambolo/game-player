//! Minimax Game Tree Search Implementation
//!
//! This module implements a game tree search using min-max strategy, alpha-beta pruning, and a transposition table. The
//! game-specific components are provided by the user using the traits defined in other modules.
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::minimax::{search, ResponseGenerator};
//!
//! // Assuming you have implemented the required traits for your game
//! let static_evaluator = MyStaticEvaluator::new();
//! let response_generator = MyResponseGenerator::new();
//! let initial_state = MyGameState::new();
//!
//! if let Some(best_move) = search(&static_evaluator, &response_generator, &initial_state, 6) {
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

static SEF_QUALITY: u32 = 0; // Quality of a value returned by the static evaluation function.

// Holds evaluation information about a response.
struct Response<S> {
    // Reference to the resulting state
    state: Rc<S>,
    // Value of the state
    value: f32,
    // Quality of the value. Quality is the number of plies searched to find the value.
    quality: u32,
}

// Holds static information pertaining to the search.
struct Context<'a, S, E: StaticEvaluator<State = S>, R: ResponseGenerator<State = S>>
where
    S: State,
{
    max_depth: u32,
    rg: &'a R,
    sef: &'a E,
    tt: RefCell<TranspositionTable>,
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
///     fn generate(&self, state: &Self::State, depth: u32) -> Vec<Self::State> {
///         // Generate all valid moves for the current player
///         let mut responses = Vec::new();
///
///         // Game-specific logic to generate moves
///         for possible_move in get_all_valid_moves(state) {
///             let new_state = state.apply_move(possible_move);
///             responses.push(new_state);
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
/// - The depth parameter can be used for depth-dependent move generation optimizations.
pub trait ResponseGenerator {
    /// The type representing game states that this generator works with
    type State: State;

    /// Generates a list of all possible responses to the given state.
    ///
    /// This method should return all legal moves available to the current player in the given state. The implementation should be
    /// game-specific and handle all rules and constraints of the particular game being played.
    ///
    /// # Arguments
    /// * `state` - The current state to generate responses for
    /// * `depth` - Current search depth (ply number), useful for optimizations
    ///
    /// # Returns
    /// A vector of game states representing all possible moves, or an empty vector if no moves are available.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let response_gen = MyResponseGenerator::new();
    /// let current_state = MyGameState::new();
    ///
    /// let possible_moves = response_gen.generate(&current_state, 0);
    /// println!("Found {} possible moves", possible_moves.len());
    /// ```
    ///
    /// # Note
    /// The caller gains ownership of the returned states.
    ///
    /// # Note
    /// Returning no responses indicates that the player cannot respond. It does not necessarily indicate that the game is over or
    /// that the player has passed. If passing is allowed, then a "pass" state should be a valid response.
    fn generate(&self, state: &Self::State, depth: u32) -> Vec<Self::State>;
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
/// `Some(S)` containing the best resulting state found, or `None` if no valid moves exist
///
/// # Examples
///
/// ```rust,ignore
/// use crate::minimax::search;
///
/// // Set up the search components
/// let evaluator = MyStaticEvaluator::new();
/// let move_generator = MyResponseGenerator::new();
/// let game_state = MyGameState::initial_position();
///
/// // Search to depth 6
/// match search(&evaluator, &move_generator, &game_state, 6) {
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
pub fn search<S, E, R>(sef: &E, rg: &R, s0: &S, max_depth: u32) -> Option<S>
where
    S: State,
    E: StaticEvaluator<State = S>,
    R: ResponseGenerator<State = S>,
{
    let context = Context {
        tt: RefCell::new(TranspositionTable::new(0)),
        sef,
        rg,
        max_depth,
    };
    let player = s0.whose_turn();
    let rc_s0 = Rc::new(s0.clone());
    if let Some(response) = search_recursive(&context, &rc_s0, -f32::INFINITY, f32::INFINITY, 1, player) {
        return Some(Rc::try_unwrap(response.state).unwrap_or_else(|rc| (*rc).clone()));
    }
    None
}

// Evaluates all of the current player's possible responses to the given state. The returned response is the one with the best value
// for that player.
fn search_recursive<S, E, R>(
    context: &Context<S, E, R>,
    state: &Rc<S>,
    mut alpha: f32,
    mut beta: f32,
    depth: u32,
    player: PlayerId,
) -> Option<Response<S>>
where
    S: State,
    E: StaticEvaluator<State = S>,
    R: ResponseGenerator<State = S>,
{
    let maximizing = player == PlayerId::Alice;

    // Quality of the value of the returned response
    let this_quality = context.max_depth.saturating_sub(depth);

    // Generate a list of the possible responses to this state. The responses are initialized with preliminary values.
    let mut responses = generate_responses(context, state, depth);

    // If there are no responses, return without a response. It's up to the caller to decide how to handle this case.
    if responses.is_empty() {
        return None;
    }

    // Sort to increase the chance of pruning earlier. For a maximizing player, sort highest to lowest to hit beta cutoffs earlier.
    // For a minimizing player, sort lowest to highest to hit alpha cutoffs earlier.
    if maximizing {
        responses.sort_by(|a, b| b.value.total_cmp(&a.value));
    } else {
        responses.sort_by(|a, b| a.value.total_cmp(&b.value));
    }

    // Evaluate each of the responses and choose the one with the best value for this player.
    let mut best_state: Option<&Rc<S>> = None;
    let mut best_value = if maximizing { -f32::INFINITY } else { f32::INFINITY };
    let mut best_quality: u32 = 0;
    let mut pruned = false;

    let wins_value = if maximizing {
        context.sef.alice_wins_value()
    } else {
        context.sef.bob_wins_value()
    };

    for response in &responses {
        // Preliminary value and quality of this response, which may be updated by search.
        let mut value = response.value;
        let mut quality = response.quality;

        // Replace the preliminary value and quality of this response with the value and quality of the opponent's subsequent
        // response via search, unless:
        // 1. The preliminary value indicates a win for this player.
        // 2. The preliminary quality is more than the quality of a search. This can be a result of obtaining the preliminary value
        //    from the result of a previous search stored in the transposition table.
        // 3. The search has reached its maximum depth.
        let preliminary_is_not_a_winner = if maximizing {
            response.value.total_cmp(&wins_value).is_lt()
        } else {
            value.total_cmp(&wins_value).is_gt()
        };

        if preliminary_is_not_a_winner && depth < context.max_depth && response.quality < this_quality {
            // Update the value by evaluating the opponent's responses. If the opponent has no response, leave the value and quality
            // as is.
            if let Some(opponent_response) = search_recursive(context, &response.state, alpha, beta, depth + 1, player.other()) {
                value = opponent_response.value;
                quality = opponent_response.quality;
            }
        }

        // Determine if this response's value is the best so far. If so, then save the value and do alpha-beta pruning.
        let is_better = if maximizing { value.total_cmp(&best_value).is_gt() } else { value.total_cmp(&best_value).is_lt() };

        if is_better {
            // Save it
            best_state = Some(&response.state);
            best_value = value;
            best_quality = quality;

            // If this player wins with this response, then there is no reason to look for anything better.
            let is_winner = if maximizing {
                best_value.total_cmp(&wins_value).is_ge()
            } else {
                best_value.total_cmp(&wins_value).is_le()
            };

            if is_winner {
                break;
            }

            // alpha-beta pruning: cutoff and bound update logic
            if maximizing {
                // Beta cutoff: if best value exceeds beta, the opponent will not allow this line.
                if best_value.total_cmp(&beta).is_gt() {
                    pruned = true;
                    break;
                }
                // Update alpha: this is the best value found so far for the maximizing player.
                if best_value.total_cmp(&alpha).is_gt() {
                    alpha = best_value;
                }
            } else {
                // Alpha cutoff: if best value is below alpha, the opponent will not allow this line.
                if best_value.total_cmp(&alpha).is_lt() {
                    pruned = true;
                    break;
                }
                // Update beta: this is the best value found so far for the minimizing player.
                if best_value.total_cmp(&beta).is_lt() {
                    beta = best_value;
                }
            }
        }
    }

    let bound_check = if maximizing {
        best_value.total_cmp(&-f32::INFINITY).is_gt()
    } else {
        best_value.total_cmp(&f32::INFINITY).is_lt()
    };
    assert!(bound_check); // Sanity check
    assert!(best_state.is_some()); // Sanity check

    // Just in case a best case was never found, return None.
    best_state.as_ref()?;

    // At this point, the value of this state becomes the value of the best response to it, and the quality becomes its quality + 1.

    // Save the value of this state in the T-table if the ply was not pruned. Pruning results in an incorrect value because the
    // search was interrupted and potentially better responses were not considered.
    if !pruned {
        context
            .tt
            .borrow_mut()
            .update(state.fingerprint(), best_value, best_quality + 1);
    }

    Some(Response::<S> {
        state: Rc::clone(best_state?),
        value: best_value,
        quality: best_quality + 1,
    })
}

// Generates a list of responses to the given node.
fn generate_responses<S, E, R>(context: &Context<S, E, R>, state: &Rc<S>, depth: u32) -> Vec<Response<S>>
where
    S: State,
    E: StaticEvaluator<State = S>,
    R: ResponseGenerator<State = S>,
{
    // Handle the case where node.state might be None.
    let responses = context.rg.generate(state.as_ref(), depth);
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

// Get a preliminary value of the state from the static evaluator or the transposition table.
fn get_preliminary_value<S, E, R>(context: &Context<S, E, R>, state: &Rc<S>) -> (f32, u32)
where
    S: State,
    E: StaticEvaluator<State = S>,
    R: ResponseGenerator<State = S>,
{
    // SEF optimization: Since any value of any state in the T-table has already been computed by search and/or SEF, it has a
    // quality that is at least as good as the quality of the value returned by the SEF. So, if the state is in the T-table, then
    // the value in the T-table is used instead of running the SEF because T-table lookup is much faster. If it is in the T-table
    // then use that value, otherwise evaluate the state and save the value.
    let fingerprint = state.fingerprint();

    // First, check if the value is in the transposition table (don't care about quality).
    let mut tt = context.tt.borrow_mut();
    if let Some(cached_value) = tt.check(fingerprint) {
        return cached_value;
    }

    // Value not in table, so evaluate with static evaluator and store result.
    let value = context.sef.evaluate(state);
    tt.update(fingerprint, value, SEF_QUALITY);

    (value, SEF_QUALITY)
}
