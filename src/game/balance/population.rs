pub struct PopulationBalance {
    /// Extra food required in storage to attempt birth: need `food >= pop + this`.
    pub increase_cost: usize,
    /// Percent chance of +1 pop per day when above food threshold and below max pop.
    pub birth_chance_percent: u8,
    /// Settlers never auto-assigned to buildings; they stay free for `g *`.
    /// `0` = all settlers can be assigned (gather fails when none free).
    /// `1` = always keep one settler for gathering when `pop >= 1`.
    pub reserve_free_settlers: usize,
}

impl Default for PopulationBalance {
    fn default() -> Self {
        Self {
            increase_cost: 2,
            birth_chance_percent: 15,
            reserve_free_settlers: 0,
        }
    }
}
