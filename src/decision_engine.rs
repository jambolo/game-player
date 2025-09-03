/// Main decision-making component using Monte Carlo Tree Search with Information Sets
pub struct HiddenGamePlayer {
    color: Color,
    name: String,
    information_state: InformationState,
    search_config: SearchConfig,
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Number of simulations to run
    pub simulation_count: usize,
    /// Maximum search depth
    pub max_depth: usize,
    /// Exploration parameter for UCB1
    pub exploration_constant: f64,
    /// Time limit for decision making
    pub time_limit_ms: u64,
}

impl HiddenGamePlayer {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            name: "HiddenGamePlayer".to_string(),
            information_state: InformationState::new(),
            search_config: SearchConfig::default(),
        }
    }

    /// Main decision function using Information Set Monte Carlo Tree Search
    fn choose_best_move(&mut self, state: &State) -> Move {
        let mut search_tree = InformationSetMCTS::new(&self.search_config);

        // Generate possible opponent hands based on current beliefs
        let opponent_hand_samples = self.sample_opponent_hands(self.search_config.simulation_count);

        // Run ISMCTS for each sampled opponent hand
        for opponent_hand in opponent_hand_samples {
            let determinized_state = state.determinize_with_opponent_hand(opponent_hand);
            search_tree.simulate(&determinized_state, self.color);
        }

        // Return the move with highest average value
        search_tree.best_move()
    }
}
