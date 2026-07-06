//! Population growth rules.

/// Birth and starvation tuning.
pub struct PopulationBalance {
    /// Extra food required in storage to attempt birth: need `food >= pop + this`.
    pub increase_cost: usize,
    /// Percent chance of +1 pop per day when above food threshold and below max pop.
    pub birth_chance_percent: u8,
    /// Births blocked while population is below this (e.g. 2 = last settler alone cannot reproduce).
    pub min_population_for_birth: usize,
    /// Consecutive hungry days (deficit > 0) before guaranteed death.
    pub starvation_days_to_death: usize,
    /// Daily death roll: `min(100, this × deficit)` when not everyone was fed.
    pub starvation_death_chance_percent: u8,
}

impl Default for PopulationBalance {
    fn default() -> Self {
        Self {
            increase_cost: 2,
            birth_chance_percent: 15,
            min_population_for_birth: 2,
            starvation_days_to_death: 2,
            starvation_death_chance_percent: 33,
        }
    }
}
