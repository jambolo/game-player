/// Models opponent behavior and playing patterns
pub struct OpponentModel {
    /// Playing style classification
    style: PlayingStyle,
    /// Preference for certain types of moves
    move_preferences: HashMap<MoveType, f64>,
    /// Adaptation rate for learning
    learning_rate: f64,
}

#[derive(Debug, Clone)]
pub enum PlayingStyle {
    Aggressive,   // Plays high-value tiles early
    Conservative, // Keeps high-value tiles for later
    Blocking,     // Focuses on blocking opponent
    Adaptive,     // Changes style based on game state
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum MoveType {
    HighValue, // Playing tiles with high pip count
    LowValue,  // Playing tiles with low pip count
    Double,    // Playing double tiles
    Block,     // Moves that limit opponent options
    Open,      // Moves that open up the board
}

impl OpponentModel {
    pub fn predict_move_probability(
        &self,
        possible_moves: &[Move],
        state: &State,
    ) -> HashMap<Move, f64> {
        let mut probabilities = HashMap::new();

        for game_move in possible_moves {
            let move_type = self.classify_move(game_move, state);
            let base_prob = self.move_preferences.get(&move_type).unwrap_or(&0.5);

            // Adjust based on game state and playing style
            let adjusted_prob = self.adjust_for_context(*base_prob, game_move, state);
            probabilities.insert(game_move.clone(), adjusted_prob);
        }

        // Normalize probabilities
        self.normalize_probabilities(probabilities)
    }

    pub fn update_from_move(&mut self, game_move: &Move, state: &State) {
        let move_type = self.classify_move(game_move, state);

        // Update preferences using reinforcement learning
        let current_pref = self.move_preferences.get(&move_type).unwrap_or(&0.5);
        let new_pref = current_pref + self.learning_rate * (1.0 - current_pref);
        self.move_preferences.insert(move_type, new_pref);

        // Update other preferences (decrease them slightly)
        for (other_type, pref) in self.move_preferences.iter_mut() {
            if *other_type != move_type {
                *pref *= (1.0 - self.learning_rate * 0.1);
            }
        }
    }
}
