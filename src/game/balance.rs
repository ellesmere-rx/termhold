/// All tunable game constants — costs, yields, worker requirements, birth rules.
///
/// Change values here (or in `Default`) to rebalance without touching logic.
pub struct Balance {
    // --- Active gathering (per `g *` command) ---
    /// Base wood before free-settler bonus.
    pub gather_wood_base: usize,
    pub gather_stone_base: usize,
    pub gather_food_base: usize,

    // --- Population growth (checked each tick if food allows) ---
    /// Extra food required in storage to attempt birth: need `food >= pop + this`.
    pub population_increase_cost: usize,
    /// Percent chance of +1 pop per day when above food threshold and below max pop.
    pub birth_chance_percent: u8,
    /// Settlers never auto-assigned to buildings; they stay free for `g *`.
    /// `0` = all settlers can be assigned (gather fails when none free).
    /// `1` = always keep one settler for gathering when `pop >= 1`.
    pub reserve_free_settlers: usize,

    // --- Hut (housing) ---
    pub hut_max_population_increase: usize,
    pub build_hut_wood_cost: usize,
    pub build_hut_stone_cost: usize,

    // --- Lumber yard ---
    pub build_lumber_yard_wood_cost: usize,
    pub build_lumber_yard_stone_cost: usize,
    /// Wood per day per **fully staffed** yard.
    pub lumber_yard_wood_production: usize,
    /// Workers required per yard; partial crew → yard does not produce.
    pub lumber_yard_max_workers: usize,

    // --- Stone quarry ---
    pub build_stone_quarry_wood_cost: usize,
    pub build_stone_quarry_stone_cost: usize,
    pub stone_quarry_stone_production: usize,
    pub stone_quarry_max_workers: usize,

    // --- Farm ---
    pub build_farm_wood_cost: usize,
    pub build_farm_stone_cost: usize,
    pub farm_food_production: usize,
    pub farm_max_workers: usize,

    // --- Barn (storage) ---
    pub barn_max_food_storage_increase: usize,
    pub build_barn_wood_cost: usize,
    pub build_barn_stone_cost: usize,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            // RESOURCES:
            // Wood
            gather_wood_base: 5,
            // Stone
            gather_stone_base: 5,
            // Food
            gather_food_base: 5,
            // Pop
            population_increase_cost: 2,
            birth_chance_percent: 15,
            reserve_free_settlers: 0,

            // BUILDINGS:
            // Hut
            build_hut_wood_cost: 10,
            build_hut_stone_cost: 10,
            hut_max_population_increase: 5,

            // Lumber
            build_lumber_yard_wood_cost: 20,
            build_lumber_yard_stone_cost: 50,
            lumber_yard_wood_production: 3,
            lumber_yard_max_workers: 2,

            // Quarry
            build_stone_quarry_wood_cost: 50,
            build_stone_quarry_stone_cost: 20,
            stone_quarry_stone_production: 2,
            stone_quarry_max_workers: 2,

            // Farm
            build_farm_wood_cost: 15,
            build_farm_stone_cost: 15,
            farm_food_production: 2,
            farm_max_workers: 2,

            // Barn
            barn_max_food_storage_increase: 15,
            build_barn_wood_cost: 20,
            build_barn_stone_cost: 30,
        }
    }
}

impl Balance {}
