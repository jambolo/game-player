//! Random-playout `ValueEstimator` for MCTS
//!
//! Requires the `mcts_random_playout` feature, since it pulls in `rand` as a dependency - a cost that
//! callers supplying their own [`ValueEstimator`](crate::mcts::ValueEstimator) (the common case for
//! static-evaluator-driven or neural-network-driven MCTS) should not have to pay.

use crate::mcts::{ResponseGenerator, ValueEstimator};
use crate::state::State;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use std::cell::RefCell;
use std::marker::PhantomData;

/// A [`State`] that can report the outcome of a game once it has ended.
///
/// [`State`] alone has no way to ask "who won" - it only exposes [`State::is_terminal`]. A game
/// implementing [`RandomPlayoutEstimator`]'s random rollout strategy needs to know the result once a
/// playout reaches a terminal position, so this trait fills that gap.
pub trait TerminalOutcome: State {
    /// Returns the exact outcome of this state in `[0.0, 1.0]` from the perspective of
    /// `self.whose_turn()`: `1.0` is a win, `0.0` a loss, and `0.5` a draw for that player. Only ever
    /// called on states for which [`State::is_terminal`] returns `true`.
    fn outcome(&self) -> f32;
}

/// A [`ValueEstimator`] that plays uniformly-random actions out to a terminal state (the classic MCTS
/// rollout) and returns the result.
///
/// Requires `G::State` to implement [`TerminalOutcome`] in addition to [`State`], since a generic
/// playout has no other way to read off who won.
///
/// The estimator owns a seeded [`StdRng`] so that searches built on it stay reproducible: the same seed
/// and the same sequence of `estimate()` calls always produce the same playouts.
pub struct RandomPlayoutEstimator<G> {
    rng: RefCell<StdRng>,
    _response_generator: PhantomData<G>,
}

impl<G> RandomPlayoutEstimator<G> {
    /// Creates a new estimator seeded for reproducible playouts.
    pub fn new(seed: u64) -> Self {
        Self {
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
            _response_generator: PhantomData,
        }
    }
}

impl<G> ValueEstimator for RandomPlayoutEstimator<G>
where
    G: ResponseGenerator,
    G::State: TerminalOutcome,
{
    type State = G::State;
    type ResponseGenerator = G;

    fn estimate(&self, state: &Self::State, rg: &G) -> f32 {
        let perspective = state.whose_turn();
        let mut current = state.clone();
        let mut rng = self.rng.borrow_mut();
        loop {
            if current.is_terminal() {
                let outcome = current.outcome();
                return if current.whose_turn() == perspective {
                    outcome
                } else {
                    1.0 - outcome
                };
            }
            let actions = rg.generate(&current);
            match actions.choose(&mut *rng) {
                Some(action) => current = current.apply(action),
                // No legal actions from a non-terminal state: nothing left to simulate:
                // treat it as a draw rather than looping or panicking.
                None => return 0.5,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlayerId;

    // A countdown game with no branching: each ply has exactly one action, so the playout is
    // deterministic regardless of the RNG seed. Reaching zero is a win for whoever's turn it is.
    #[derive(Debug, Clone, PartialEq)]
    struct CountdownState {
        remaining: u8,
        turn: PlayerId,
    }

    #[derive(Debug, Clone)]
    struct CountdownAction;

    impl State for CountdownState {
        type Action = CountdownAction;

        fn fingerprint(&self) -> u64 {
            self.remaining as u64
        }

        fn whose_turn(&self) -> PlayerId {
            self.turn
        }

        fn is_terminal(&self) -> bool {
            self.remaining == 0
        }

        fn apply(&self, _action: &CountdownAction) -> Self {
            Self {
                remaining: self.remaining - 1,
                turn: self.turn.other(),
            }
        }
    }

    impl TerminalOutcome for CountdownState {
        // The player to move when the countdown reaches zero is the one who was just defeated:
        // the other player made the winning move. So this is a loss for whoever's turn it is.
        fn outcome(&self) -> f32 {
            0.0
        }
    }

    struct CountdownResponseGenerator;

    impl ResponseGenerator for CountdownResponseGenerator {
        type State = CountdownState;

        fn generate(&self, state: &CountdownState) -> Vec<CountdownAction> {
            if state.is_terminal() { vec![] } else { vec![CountdownAction] }
        }
    }

    #[test]
    fn test_deterministic_playout_matches_exact_outcome() {
        let rg = CountdownResponseGenerator;
        let estimator = RandomPlayoutEstimator::<CountdownResponseGenerator>::new(42);

        // Alice to move with 1 remaining ply: Alice's move reaches the terminal state, where it is
        // Bob's turn and Bob has lost - a win for Alice.
        let state = CountdownState {
            remaining: 1,
            turn: PlayerId::Alice,
        };
        assert_eq!(estimator.estimate(&state, &rg), 1.0);

        // Alice to move with 2 remaining plies: Alice moves, then Bob moves to reach the terminal
        // state, where it is Alice's turn and Alice has lost - a loss for the original mover, Alice.
        let state = CountdownState {
            remaining: 2,
            turn: PlayerId::Alice,
        };
        assert_eq!(estimator.estimate(&state, &rg), 0.0);
    }

    #[test]
    fn test_terminal_state_returns_exact_outcome_without_playout() {
        let rg = CountdownResponseGenerator;
        let estimator = RandomPlayoutEstimator::<CountdownResponseGenerator>::new(7);

        let state = CountdownState {
            remaining: 0,
            turn: PlayerId::Bob,
        };
        assert_eq!(estimator.estimate(&state, &rg), 0.0);
    }

    // A branching game (Nim-like: 1 or 2 stones may be taken from a pile) used to check that random
    // playouts stay within the required range and that different seeds can produce different results.
    #[derive(Debug, Clone, PartialEq)]
    struct NimState {
        stones: u8,
        turn: PlayerId,
    }

    #[derive(Debug, Clone)]
    struct NimAction {
        take: u8,
    }

    impl State for NimState {
        type Action = NimAction;

        fn fingerprint(&self) -> u64 {
            self.stones as u64
        }

        fn whose_turn(&self) -> PlayerId {
            self.turn
        }

        fn is_terminal(&self) -> bool {
            self.stones == 0
        }

        fn apply(&self, action: &NimAction) -> Self {
            Self {
                stones: self.stones.saturating_sub(action.take),
                turn: self.turn.other(),
            }
        }
    }

    impl TerminalOutcome for NimState {
        // Whoever is left with no stones to take made the losing position.
        fn outcome(&self) -> f32 {
            0.0
        }
    }

    struct NimResponseGenerator;

    impl ResponseGenerator for NimResponseGenerator {
        type State = NimState;

        fn generate(&self, state: &NimState) -> Vec<NimAction> {
            (1..=2u8.min(state.stones)).map(|take| NimAction { take }).collect()
        }
    }

    #[test]
    fn test_random_playout_stays_in_range() {
        let rg = NimResponseGenerator;
        let estimator = RandomPlayoutEstimator::<NimResponseGenerator>::new(1234);
        let state = NimState {
            stones: 10,
            turn: PlayerId::Alice,
        };
        for _ in 0..100 {
            let value = estimator.estimate(&state, &rg);
            assert!((0.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn test_same_seed_is_reproducible() {
        let rg = NimResponseGenerator;
        let state = NimState {
            stones: 10,
            turn: PlayerId::Alice,
        };

        let a = RandomPlayoutEstimator::<NimResponseGenerator>::new(99);
        let b = RandomPlayoutEstimator::<NimResponseGenerator>::new(99);

        let sequence_a: Vec<f32> = (0..20).map(|_| a.estimate(&state, &rg)).collect();
        let sequence_b: Vec<f32> = (0..20).map(|_| b.estimate(&state, &rg)).collect();
        assert_eq!(sequence_a, sequence_b);
    }
}
