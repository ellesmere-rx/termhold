mod buildings;
mod gather;
mod population;

pub use buildings::BuildingsBalance;
pub use gather::GatherBalance;
pub use population::PopulationBalance;

/// All tunable game constants — costs, yields, worker requirements, birth rules.
///
/// Change values here (or in sub-module `Default`) to rebalance without touching logic.
#[derive(Default)]
pub struct Balance {
    pub gather: GatherBalance,
    pub population: PopulationBalance,
    pub buildings: BuildingsBalance,
}
