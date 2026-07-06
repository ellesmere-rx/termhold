//! Buildings: kinds, instances, worker slots, and the colony's building list.
//!
//! Each production building is a separate instance with its own worker count.
//! How many workers a building *needs* to produce comes from [`BuildingsBalance`]
//! via [`BuildingKind::workers_required`] — not stored on the instance.

use super::balance::BuildingsBalance;

/// All buildable structure types in the colony.
///
/// - **Hut** / **Barn** — infrastructure (housing, food storage); no workers.
/// - **Farm** / **LumberYard** / **StoneQuarry** — production; require assigned settlers
///   to generate passive income each day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingKind {
    /// Increases [`super::Colony::max_population`].
    Hut,
    /// Increases [`super::Colony::max_food`] (barn storage cap).
    Barn,
    /// Passive food per day when fully staffed.
    Farm,
    /// Passive wood per day when fully staffed.
    LumberYard,
    /// Passive stone per day when fully staffed.
    StoneQuarry,
}

/// One built structure in the colony.
///
/// Multiple instances of the same [`BuildingKind`] may exist (e.g. two farms).
/// Worker assignment is per instance; the player sets totals by type (`w farm 3`),
/// which [`BuildingList::set_workers_on_kind`] distributes in list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Building {
    /// What this instance is.
    pub kind: BuildingKind,
    /// Settlers currently working here. Always `0` for hut/barn.
    /// Capped at [`BuildingKind::workers_required`] after [`BuildingList::clamp_workers`].
    pub assigned: usize,
}

/// All building instances owned by a colony.
///
/// Stored as a flat [`Vec`] in construction order. Worker distribution walks this order.
#[derive(Debug, Clone, Default)]
pub struct BuildingList {
    items: Vec<Building>,
}

impl BuildingKind {
    /// Production types in worker clamp priority: farm first, then lumber, then quarry.
    pub const PRODUCTION: [Self; 3] = [Self::Farm, Self::LumberYard, Self::StoneQuarry];

    /// Short CLI / log label (`farm`, `lumber`, `quarry`, …).
    pub fn label(self) -> &'static str {
        match self {
            Self::Hut => "hut",
            Self::Barn => "barn",
            Self::Farm => "farm",
            Self::LumberYard => "lumber",
            Self::StoneQuarry => "quarry",
        }
    }

    /// Whether settlers can be assigned to buildings of this kind.
    pub fn employs_workers(self) -> bool {
        matches!(self, Self::Farm | Self::LumberYard | Self::StoneQuarry)
    }

    /// Workers required on **one** instance before it counts as staffed and produces.
    ///
    /// Read from balance (`farm_max_workers`, …). Partial crew → no output for that building.
    pub fn workers_required(self, balance: &BuildingsBalance) -> usize {
        match self {
            Self::Farm => balance.farm_max_workers,
            Self::LumberYard => balance.lumber_yard_max_workers,
            Self::StoneQuarry => balance.stone_quarry_max_workers,
            _ => 0,
        }
    }

    /// Wood cost to construct one instance.
    pub fn build_wood_cost(self, balance: &BuildingsBalance) -> usize {
        match self {
            Self::Hut => balance.build_hut_wood_cost,
            Self::Barn => balance.build_barn_wood_cost,
            Self::Farm => balance.build_farm_wood_cost,
            Self::LumberYard => balance.build_lumber_yard_wood_cost,
            Self::StoneQuarry => balance.build_stone_quarry_wood_cost,
        }
    }

    /// Stone cost to construct one instance.
    pub fn build_stone_cost(self, balance: &BuildingsBalance) -> usize {
        match self {
            Self::Hut => balance.build_hut_stone_cost,
            Self::Barn => balance.build_barn_stone_cost,
            Self::Farm => balance.build_farm_stone_cost,
            Self::LumberYard => balance.build_lumber_yard_stone_cost,
            Self::StoneQuarry => balance.build_stone_quarry_stone_cost,
        }
    }

    /// Resources produced per day by **one fully staffed** instance.
    pub fn passive_output(self, balance: &BuildingsBalance) -> usize {
        match self {
            Self::Farm => balance.farm_food_production,
            Self::LumberYard => balance.lumber_yard_wood_production,
            Self::StoneQuarry => balance.stone_quarry_stone_production,
            _ => 0,
        }
    }
}

impl Building {
    /// `true` when [`Self::assigned`] meets or exceeds [`BuildingKind::workers_required`].
    /// Only staffed buildings contribute to passive income.
    pub fn is_staffed(self, balance: &BuildingsBalance) -> bool {
        self.assigned >= self.kind.workers_required(balance)
    }
}

impl BuildingList {
    /// Number of built instances of `kind`.
    pub fn count(&self, kind: BuildingKind) -> usize {
        self.items.iter().filter(|b| b.kind == kind).count()
    }

    /// Add a new instance with zero assigned workers.
    pub fn add(&mut self, kind: BuildingKind) {
        self.items.push(Building { kind, assigned: 0 });
    }

    /// Whether the colony has at least one farm, lumber yard, or quarry.
    pub fn has_production(&self) -> bool {
        BuildingKind::PRODUCTION
            .iter()
            .any(|kind| self.count(*kind) > 0)
    }

    /// Sum of [`Building::assigned`] across all instances of `kind`.
    pub fn workers_on(&self, kind: BuildingKind) -> usize {
        self.items
            .iter()
            .filter(|b| b.kind == kind)
            .map(|b| b.assigned)
            .sum()
    }

    /// Total settlers assigned to any building (all kinds).
    pub fn total_assigned(&self) -> usize {
        self.items.iter().map(|b| b.assigned).sum()
    }

    /// Maximum assignable workers for `kind`: `count × workers_required`.
    pub fn workers_needed(&self, kind: BuildingKind, balance: &BuildingsBalance) -> usize {
        self.count(kind) * kind.workers_required(balance)
    }

    /// Count of instances of `kind` that are fully staffed ([`Building::is_staffed`]).
    pub fn staffed(&self, kind: BuildingKind, balance: &BuildingsBalance) -> usize {
        self.items
            .iter()
            .filter(|b| b.kind == kind && b.is_staffed(balance))
            .count()
    }

    /// Set the **total** workers on all instances of `kind`.
    ///
    /// Fills instances in [`Self::items`] order: each gets up to `workers_required`,
    /// then overflow goes to the next. Example: 2 farms (need 2 each), `count = 3`
    /// → first farm 2, second farm 1.
    pub fn set_workers_on_kind(
        &mut self,
        kind: BuildingKind,
        count: usize,
        balance: &BuildingsBalance,
    ) {
        let required = kind.workers_required(balance);
        let mut remaining = count;
        for building in &mut self.items {
            if building.kind != kind {
                continue;
            }
            let assign = remaining.min(required);
            building.assigned = assign;
            remaining -= assign;
        }
    }

    /// Cap each building at its required crew, then drop assignments if total exceeds `population`.
    ///
    /// Unassign order: quarry → lumber → farm, last instance in list first within each kind.
    pub fn clamp_workers(&mut self, balance: &BuildingsBalance, population: usize) {
        for building in &mut self.items {
            if building.kind.employs_workers() {
                let cap = building.kind.workers_required(balance);
                building.assigned = building.assigned.min(cap);
            }
        }

        while self.total_assigned() > population {
            let Some(index) = Self::index_to_unassign(&self.items) else {
                break;
            };
            self.items[index].assigned -= 1;
        }
    }

    /// Index of one worker to remove when total assigned exceeds population.
    fn index_to_unassign(items: &[Building]) -> Option<usize> {
        for kind in BuildingKind::PRODUCTION.iter().rev() {
            if let Some(index) = items
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, b)| (b.kind == *kind && b.assigned > 0).then_some(i))
            {
                return Some(index);
            }
        }
        None
    }
}
