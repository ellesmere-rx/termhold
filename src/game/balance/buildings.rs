//! Build costs, worker slots per instance, and passive output rates.

/// Tunable constants for all building types.
///
/// Worker fields (`*_max_workers`) define how many settlers one instance needs
/// to be considered staffed; logic reads them via [`BuildingKind::workers_required`].
pub struct BuildingsBalance {
    // --- Hut (housing) ---
    /// Population cap added per hut built.
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
    /// Food storage cap added per barn built.
    pub barn_max_food_storage_increase: usize,
    pub build_barn_wood_cost: usize,
    pub build_barn_stone_cost: usize,
}

impl Default for BuildingsBalance {
    fn default() -> Self {
        Self {
            hut_max_population_increase: 3,
            build_hut_wood_cost: 12,
            build_hut_stone_cost: 12,

            build_lumber_yard_wood_cost: 12,
            build_lumber_yard_stone_cost: 15,
            lumber_yard_wood_production: 5,
            lumber_yard_max_workers: 2,

            build_stone_quarry_wood_cost: 18,
            build_stone_quarry_stone_cost: 18,
            stone_quarry_stone_production: 4,
            stone_quarry_max_workers: 2,

            build_farm_wood_cost: 18,
            build_farm_stone_cost: 18,
            farm_food_production: 2,
            farm_max_workers: 2,

            barn_max_food_storage_increase: 15,
            build_barn_wood_cost: 32,
            build_barn_stone_cost: 32,
        }
    }
}
