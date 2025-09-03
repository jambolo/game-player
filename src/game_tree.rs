//! Generic Game Tree Search Implementation
//!
//! This module implements a game tree search using min-max strategy, alpha-beta pruning, and a transposition table. The
//! game-specific components are provided at run-time.

use std::sync::{Arc, Mutex};
use std::marker::PhantomData;

use crate::game_state::{GameState, PlayerId};
use crate::static_evaluator::StaticEvaluator;
use crate::transposition_table::TranspositionTable;

static SEF_QUALITY: i16 = 0; // Quality of a value returned by the static evaluation function

// Struct for holding information about a game state response.
struct Response<G: GameState> {
    // Reference to the responding game state
    state: Arc<G>,
    // Value of the responding state
    value: f32,
    // Quality of the value. Quality is the number of plies searched to find the value.
    quality: i16,
}

/// Response generator function object type.
/// 
/// # Arguments
/// * `state` - state to respond to
/// * `depth` - current ply
/// 
/// # Returns
/// list of all possible responses
/// 
/// # Note
/// The caller gains ownership of the returned states.
/// 
/// # Note
/// Returning no responses simply indicates that the player cannot respond. It does not necessarily indicate that the game is
/// over or that the player has passed. If passing is allowed, then a pass should be a valid response.

/// A game tree search implementation using minimax strategy, alpha-beta pruning, and a transposition table.
pub struct GameTree<G: GameState, E: StaticEvaluator<G>, R>
where
    R: Fn(&Arc<G>, i32) -> Vec<Box<G>>,
{
    /// Transposition table (persistent)
    transposition_table: Arc<Mutex<TranspositionTable>>,
    /// Static evaluation function (persistent)
    static_evaluator: Arc<E>,
    /// Response generator (persistent)
    response_generator: R,
    /// How many plies to search
    max_depth: i32,
    /// Phantom data to indicate G is used
    _phantom: PhantomData<G>,
}

impl<G: GameState, E: StaticEvaluator<G>, R> GameTree<G, E, R> 
where
    R: Fn(&Arc<G>, i32) -> Vec<Box<G>>,
{
    /// Creates a new game tree.
    ///
    /// # Arguments
    /// * `tt` - A transposition table to be used in a search. The table is assumed to be persistent.
    /// * `sef` - The static evaluation function
    /// * `rg` - The response generator
    /// * `max_depth` - The maximum number of plies to search
    pub fn new(tt: &Arc<Mutex<TranspositionTable>>, sef: &Arc<E>, rg: R, max_depth: i32) -> Self {
        Self {
            transposition_table: Arc::clone(&tt),
            static_evaluator: Arc::clone(&sef),
            response_generator: rg,
            max_depth,
            _phantom: PhantomData,
        }
    }

    /// Searches for the best response to the given state.
    ///
    /// # Arguments
    /// * `s0` - The current state
    ///
    /// # Returns
    /// A reference to the best response's state.
    pub fn find_best_response(&self, s0: &Arc<G>) -> Option<Arc<G>> {
        if s0.whose_turn() == PlayerId::ALICE as u8 {
            if let Some(response) = self.alice_search(s0, -f32::INFINITY, f32::INFINITY, 0) {
                return Some(Arc::clone(&response.state));
            }
        } else {
            if let Some(response) = self.bob_search(s0, -f32::INFINITY, f32::INFINITY, 0) {
                return Some(Arc::clone(&response.state));
            }
        }
        None
    }

    // Evaluates all of Alice's possible responses to the given state. The returned response is the one with the highest value.
    fn alice_search(&self, state: &Arc<G>, mut alpha: f32, beta: f32, depth: i32) -> Option<Response<G>> {
        // Depth of responses to this state
        let response_depth = depth + 1;
        // Quality of a response as a result of a search at this depth.
        let search_quality = (self.max_depth - response_depth) as i16;

        // Generate a list of the possible responses to this state by Alice. The responses are initialized with preliminary values.
        let mut responses = self.generate_responses(state, depth);

        // If there are no responses, return without a response. It's up to the caller to decide how to handle this case.
        if responses.is_empty() {
            return None;
        }

        // Sort from highest to lowest in order to increase the chance of triggering a beta cutoff earlier.
        responses.sort_by(|a, b| {
            b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Evaluate each of the responses and choose the one with the highest value
        let mut best_state: Option<Arc<G>> = None;
        let mut best_value = -f32::INFINITY;
        let mut best_quality = -1;
        let mut pruned = false;
        
        for mut response in responses {
            // Replace the preliminary value and quality of this response with the value and quality of Bob's subsequent response
            // to it.
            // The following conditions will cause the search to be skipped:
            // 1. The preliminary value indicates a win for Alice.
            // 2. The preliminary quality is more than the quality of a search. This can be a result of obtaining the preliminary
            //    value from the result of a previous search stored in the transposition table.
            // 3. The search has reached its maximum depth.
            if response.value < self.static_evaluator.alice_wins_value() &&
                response_depth < self.max_depth &&
                response.quality < search_quality
            {
                // Update the value of Alice's response by evaluating Bob's responses to it. If Bob has no response, then
                // leave the response's value and quality as is.
                if let Some(bob_response) = self.bob_search(&response.state, alpha, beta, response_depth) {
                    response.value = bob_response.value;
                    response.quality = bob_response.quality;
                }
            }

            // Determine if this response's value is the best so far. If so, then save the value and do alpha-beta pruning
            if response.value > best_value {
                // Save it
                best_state = Some(Arc::clone(&response.state));
                best_value = response.value;
                best_quality = response.quality;

                // If Alice wins with this response, then there is no reason to look for anything better
                if best_value >= self.static_evaluator.alice_wins_value() {
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
        if best_state.is_none() {
            return None;
        }

        // At this point, the value of this state becomes the value of the best response to it, and the quality becomes its
        // quality + 1.
        // 
        // Save the value of this state in the T-table if the ply was not pruned. Pruning results in an incorrect value because the
        // search was interrupted and potentially better responses were not considered.
        if !pruned {
            self.transposition_table.lock().unwrap().update(state.fingerprint(), best_value, best_quality + 1);
        }

        Some(Response::<G> {
            state: best_state?,
            value: best_value,
            quality: best_quality + 1,
        })
    }

    // Evaluates all of Bob's possible responses to the given state. The returned response is the one with the lowest value.
    fn bob_search(&self, state: &Arc<G>, alpha: f32, mut beta: f32, depth: i32) -> Option<Response<G>> {
        // Depth of responses to this state
        let response_depth = depth + 1;
        // Quality of a response as a result of a search at this depth.
        let search_quality = (self.max_depth - response_depth) as i16;

        // Generate a list of the possible responses to this state by Bob. The responses are initialized with preliminary values.
        let mut responses = self.generate_responses(state, depth);

        // If there are no responses, return without a response. It's up to the caller to decide how to handle this case.
        if responses.is_empty() {
            return None;
        }

        // Sort from lowest to highest in order to increase the chance of triggering an alpha cutoff earlier
        responses.sort_by(|a, b| {
            a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Evaluate each of the responses and choose the one with the lowest value
        let mut best_state: Option<Arc<G>> = None;
        let mut best_value = f32::INFINITY;
        let mut best_quality = -1;
        let mut pruned = false;
        
        for mut response in responses {
            // Replace the preliminary value and quality of this response with the value and quality of Alice's subsequent response
            // to it.
            // The following conditions will cause the search to be skipped:
            // 1. The preliminary value indicates a win for Bob.
            // 2. The preliminary quality is more than the quality of a search. This can be a result of obtaining the preliminary
            //    value from the result of a previous search stored in the transposition table.
            // 3. The search has reached its maximum depth.
            if response.value > self.static_evaluator.bob_wins_value() &&
                response_depth < self.max_depth &&
                response.quality < search_quality
            {
                // Update the value of Bob's response by evaluating Alice's responses to it. If Alice has no response, then
                // leave the response's value and quality as is.
                if let Some(alice_response) = self.alice_search(&response.state, alpha, beta, response_depth) {
                    response.value = alice_response.value;
                    response.quality = alice_response.quality;
                }
            }

            // Determine if this response's value is the best so far. If so, then save the value and do alpha-beta pruning
            if response.value < best_value {
                // Save it
                best_state = Some(Arc::clone(&response.state));
                best_value = response.value;
                best_quality = response.quality;

                // If Bob wins with this response, then there is no reason to look for anything better
                if best_value <= self.static_evaluator.bob_wins_value() {
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
                // value of this response is lower than the beta, then it is a better response for Bob. The beta is
                // subsequently passed to Alice's search so that if Alice finds a response with a higher value than the beta, then
                // there is no reason to continue because Bob already has a better response and will choose it instead of allowing
                // Alice to make a move with a higher value.
                if best_value < beta {
                    beta = best_value;
                }
            }
        }

        assert!(best_value < f32::INFINITY); // Sanity check
        assert!(best_quality >= 0); // Sanity check
        assert!(best_state.is_some()); // Sanity check

        // Just in case
        if best_state.is_none() {
            return None;
        }

        // At this point, the value of this state becomes the value of the best response to it, and the quality becomes its
        // quality + 1.
        // 
        // Save the value of this state in the T-table if the ply was not pruned. Pruning results in an incorrect value because the
        // search was interrupted and potentially better responses were not considered.
        if !pruned {
            self.transposition_table.lock().unwrap().update(state.fingerprint(), best_value, best_quality + 1);
        }

        Some(Response::<G> {
            state: best_state?,
            value: best_value,
            quality: best_quality + 1,
        })
    }

    // Generates a list of responses to the given node
    fn generate_responses(&self, state: &Arc<G>, depth: i32) -> Vec<Response<G>> {
        // Handle the case where node.state might be None
        let responses = (self.response_generator)(state, depth);
        responses.into_iter().map(|state| {
            let arc_state = Arc::from(state);
            let (value, quality) = self.get_preliminary_value(&arc_state);
            Response::<G> {
                state: arc_state,
                value,
                quality,
            }
        }).collect()
    }

    // Get a preliminary value of the state from the static evaluator or the transposition table
    fn get_preliminary_value(&self, state: &Arc<G>) -> (f32, i16) {
        // SEF optimization:
        // Since any value of any state in the T-table has already been computed by search and/or SEF, it has a quality that is at
        // least as good as the quality of the value returned by the SEF. So, if the state being evaluated is in the T-table, then
        // the value in the T-table is used instead of running the SEF because T-table lookup is so much faster than the SEF.

        // If it is in the T-table then use that value, otherwise compute the value using SEF.
        if let Some(result) = self.transposition_table.lock().unwrap().check(state.fingerprint(), -1) {
            (result.value, result.quality)
        }
        else {
            let value = self.static_evaluator.evaluate(state);
            // Save the value of the state in the T-table
            self.transposition_table.lock().unwrap().update(state.fingerprint(), value, SEF_QUALITY);
            (value, SEF_QUALITY)
        }
    }
}
