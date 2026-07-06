//! Colony state: resources, population, buildings, and all player-facing rules.
//!
//! [`Colony`] is the domain model. UI parses input into [`Actions`](super::Actions);
//! [`Game`](super::Game) applies actions and runs the daily tick.

use super::ResourceKind;
use super::balance::Balance;
use super::building::{BuildingKind, BuildingList};

/// The player's settlement: inventory, population caps, and building instances.
pub struct Colony {
    /// Display name (UI / logs).
    pub name: String,
    /// Current wood stockpile.
    pub wood: usize,
    /// Current stone stockpile.
    pub stone: usize,
    /// Current food in storage (subject to [`Self::max_food`] and daily consumption).
    pub food: usize,
    /// Living settlers. Each assigned worker counts against this; gather uses the rest.
    pub population: usize,
    /// Housing cap; increased by huts.
    pub max_population: usize,
    /// Food storage cap; increased by barns. Excess food spoils at end of tick.
    pub max_food: usize,
    /// Current days of starvation before death.
    pub starvation_days: usize,
    /// All built structures and their per-instance worker assignments.
    pub buildings: BuildingList,
}

impl Colony {
    /// Built instances of `kind`.
    pub fn count(&self, kind: BuildingKind) -> usize {
        self.buildings.count(kind)
    }

    /// Total settlers assigned to all instances of `kind`.
    pub fn workers_on(&self, kind: BuildingKind) -> usize {
        self.buildings.workers_on(kind)
    }

    /// Maximum workers assignable to `kind` (sum of slots on all instances).
    pub fn workers_needed(&self, kind: BuildingKind, balance: &Balance) -> usize {
        self.buildings.workers_needed(kind, &balance.buildings)
    }

    /// How many instances of `kind` are fully staffed and producing.
    pub fn staffed(&self, kind: BuildingKind, balance: &Balance) -> usize {
        self.buildings.staffed(kind, &balance.buildings)
    }

    /// Settlers not assigned to any building — only these can [`Self::gather`].
    pub fn free_workers(&self) -> usize {
        self.population
            .saturating_sub(self.buildings.total_assigned())
    }

    /// Total settlers assigned across all buildings.
    pub fn assigned_workers(&self) -> usize {
        self.buildings.total_assigned()
    }

    /// `w <kind> <count>`: set total workers on a production type.
    ///
    /// Validates against slot capacity and total population (including other kinds).
    /// Distributes across instances via [`BuildingList::set_workers_on_kind`].
    pub fn set_workers(
        &mut self,
        kind: BuildingKind,
        count: usize,
        balance: &Balance,
    ) -> Result<(), &'static str> {
        if !kind.employs_workers() {
            return Err("That building does not use workers.");
        }

        let max = self.workers_needed(kind, balance);
        if count > max {
            return Err("Too many workers for that building type.");
        }

        let other: usize = BuildingKind::PRODUCTION
            .iter()
            .filter(|k| **k != kind)
            .map(|k| self.workers_on(*k))
            .sum();
        if other + count > self.population {
            return Err("Not enough settlers — lower another assignment or grow population.");
        }

        self.buildings
            .set_workers_on_kind(kind, count, &balance.buildings);
        Ok(())
    }

    /// Trim invalid assignments after population drop or new production building.
    pub fn clamp_workers(&mut self, balance: &Balance) {
        self.buildings
            .clamp_workers(&balance.buildings, self.population);
    }

    /// Human-readable warnings for production buildings that are built but not fully staffed.
    pub fn understaffed_messages(&self, balance: &Balance) -> Vec<String> {
        let mut messages = Vec::new();

        for kind in BuildingKind::PRODUCTION {
            let total = self.count(kind);
            if total == 0 {
                continue;
            }
            let needed = self.workers_needed(kind, balance);
            let assigned = self.workers_on(kind);
            if assigned >= needed {
                continue;
            }
            let idle = total - self.staffed(kind, balance);
            let per_building = kind.workers_required(&balance.buildings);
            let (name, unit) = match kind {
                BuildingKind::Farm => ("farm", "farm"),
                BuildingKind::LumberYard => ("lumber yard", "yard"),
                BuildingKind::StoneQuarry => ("quarry", "quarry"),
                _ => continue,
            };
            messages.push(format!(
                "{idle} {name}(s) idle: {assigned}/{needed} workers assigned (need {per_building} per {unit})"
            ));
        }

        messages
    }

    /// Expected gather yield for `kind` from current free settlers (does not mutate state).
    pub fn gather_yield(&self, kind: ResourceKind, balance: &Balance) -> usize {
        let free = self.free_workers();
        if free == 0 {
            return 0;
        }
        Self::yield_from_pop(balance.gather.base(kind), free, kind.gather_percent())
    }

    pub fn wood_yield(&self, balance: &Balance) -> usize {
        self.gather_yield(ResourceKind::Wood, balance)
    }

    pub fn stone_yield(&self, balance: &Balance) -> usize {
        self.gather_yield(ResourceKind::Stone, balance)
    }

    pub fn food_yield(&self, balance: &Balance) -> usize {
        self.gather_yield(ResourceKind::Food, balance)
    }

    /// Active gathering (`g wood`, etc.): uses free settlers only; advances day separately in UI.
    pub fn gather(&mut self, kind: ResourceKind, balance: &Balance) -> Result<usize, &'static str> {
        if self.free_workers() == 0 {
            return Err(
                "No free settlers — everyone works at buildings. Grow population or leave fewer workers assigned.",
            );
        }

        let gather_yield = match kind {
            ResourceKind::Food => self
                .gather_yield(kind, balance)
                .min(self.max_food.saturating_sub(self.food)),
            _ => self.gather_yield(kind, balance),
        };

        match kind {
            ResourceKind::Wood => self.wood += gather_yield,
            ResourceKind::Stone => self.stone += gather_yield,
            ResourceKind::Food => self.food += gather_yield,
        }

        Ok(gather_yield)
    }

    /// Construct one building: pay costs, add instance, apply hut/barn cap bonuses.
    pub fn build(&mut self, kind: BuildingKind, balance: &Balance) -> Result<usize, &'static str> {
        let wood_cost = kind.build_wood_cost(&balance.buildings);
        let stone_cost = kind.build_stone_cost(&balance.buildings);

        if self.wood < wood_cost {
            return Err("Not enough wood for this building!");
        }
        if self.stone < stone_cost {
            return Err("Not enough stone for this building!");
        }

        self.wood -= wood_cost;
        self.stone -= stone_cost;
        self.buildings.add(kind);

        match kind {
            BuildingKind::Hut => {
                self.max_population += balance.buildings.hut_max_population_increase;
            }
            BuildingKind::Barn => {
                self.max_food += balance.buildings.barn_max_food_storage_increase;
            }
            _ => {}
        }

        Ok(1)
    }

    /// Daily passive wood from fully staffed lumber yards.
    pub fn passive_wood(&self, balance: &Balance) -> usize {
        self.staffed(BuildingKind::LumberYard, balance)
            * BuildingKind::LumberYard.passive_output(&balance.buildings)
    }

    /// Daily passive stone from fully staffed quarries.
    pub fn passive_stone(&self, balance: &Balance) -> usize {
        self.staffed(BuildingKind::StoneQuarry, balance)
            * BuildingKind::StoneQuarry.passive_output(&balance.buildings)
    }

    /// Daily passive food from fully staffed farms.
    pub fn passive_food(&self, balance: &Balance) -> usize {
        self.staffed(BuildingKind::Farm, balance)
            * BuildingKind::Farm.passive_output(&balance.buildings)
    }

    /// Apply end-of-day production; food gain is capped by remaining storage.
    /// Returns `(wood, stone, food_added, food_lost)`.
    pub fn apply_passive_income(&mut self, balance: &Balance) -> (usize, usize, usize, usize) {
        let wood = self.passive_wood(balance);
        let stone = self.passive_stone(balance);

        let food_gain = self.passive_food(balance);
        let food = food_gain.min(self.max_food.saturating_sub(self.food));
        let food_lost = food_gain.saturating_sub(food);
        self.food += food;
        self.wood += wood;
        self.stone += stone;
        (wood, stone, food, food_lost)
    }

    /// Gather formula: `base + pop × percent / 100`.
    pub fn yield_from_pop(base: usize, pop: usize, percent: usize) -> usize {
        base + pop * percent / 100
    }
}

impl Default for Colony {
    fn default() -> Self {
        let mut buildings = BuildingList::default();
        buildings.add(BuildingKind::Hut);

        Self {
            name: "Default colony".to_string(),
            wood: 50,
            stone: 30,
            food: 20,
            population: 5,
            max_population: 5,
            max_food: 25,
            starvation_days: 0,
            buildings,
        }
    }
}
