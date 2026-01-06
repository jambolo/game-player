/// Information Set Monte Carlo Tree Search implementation
pub struct InformationSetMCTS {
    tree: HashMap<GameStateKey, MCTSNode>,
    config: SearchConfig,
}

#[derive(Debug, Clone)]
pub struct MCTSNode {
    visits: usize,
    total_value: f64,
    children: HashMap<Move, GameStateKey>,
    untried_moves: Vec<Move>,
}

impl InformationSetMCTS {
    pub fn simulate(&mut self, state: &State, player_color: Color) -> f64 {
        let state_key = state.to_key();

        // Selection phase
        let (final_state, path) = self.select_and_expand(&state_key, state);

        // Simulation phase (random playout)
        let result = self.rollout(&final_state, player_color);

        // Backpropagation phase
        self.backpropagate(&path, result);

        result
    }

    fn select_and_expand(
        &mut self,
        state_key: &GameStateKey,
        state: &State,
    ) -> (State, Vec<GameStateKey>) {
        let mut current_state = state.clone();
        let mut path = vec![state_key.clone()];

        loop {
            let node = self
                .tree
                .entry(state_key.clone())
                .or_insert_with(|| MCTSNode::new(&current_state));

            if !node.untried_moves.is_empty() {
                // Expansion phase
                let move_to_try = node.untried_moves.pop().unwrap();
                let new_state = current_state.apply_move(&move_to_try);
                let new_key = new_state.to_key();

                node.children.insert(move_to_try, new_key.clone());
                path.push(new_key);

                return (new_state, path);
            } else if !node.children.is_empty() {
                // Selection phase using UCB1
                let best_move = self.select_best_child(node);
                let new_key = node.children[&best_move].clone();
                current_state = current_state.apply_move(&best_move);
                path.push(new_key.clone());
            } else {
                // Terminal node
                return (current_state, path);
            }
        }
    }
}
