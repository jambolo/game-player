/// Updates beliefs about opponent state based on new information
impl InformationState {
    pub fn update_on_opponent_move(&mut self, game_move: &Move, state: &State) {
        // Update known tiles
        if let Move::PlayTile(tile) = game_move {
            self.known_opponent_tiles.remove(tile);
            self.possible_opponent_tiles.remove(tile);
        }

        // Add move to history for pattern analysis
        self.opponent_move_history.push(game_move.clone());

        // Update probability distributions
        self.update_hand_probabilities(game_move, state);

        // Infer new constraints
        self.infer_constraints(game_move, state);
    }

    pub fn update_on_opponent_pass(&mut self, state: &State) {
        // Opponent passed - they don't have any playable tiles
        let playable_numbers = state.get_playable_numbers();
        for number in playable_numbers {
            self.constraints.push(HandConstraint::CannotPlay(number));
        }

        // Update probability distributions based on this constraint
        self.prune_impossible_hands();
    }
}
