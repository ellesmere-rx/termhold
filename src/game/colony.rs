/// All mutable colony state: resources, caps, building counts, worker assignments.
///
/// Worker fields (`workers_on_*`) hold the current assignment (manual or from auto-assign).
use super::balance::Balance;
use super::commands::WorkerSite;
use super::ResourceKind;

pub struct Colony {
    pub name: String,

    // --- Resources (current amounts) ---
    pub wood: usize,
    pub stone: usize,
    pub food: usize,

    // --- Population ---
    /// Living settlers. Each eats 1 food per day in [`Game::tick`].
    pub population: usize,
    /// Raised by huts. Births only happen below this cap.
    pub max_population: usize,

    // --- Food storage ---
    /// Current food cannot exceed this after spoilage at end of tick.
    pub max_food: usize,

    // --- Buildings (counts only; no per-building IDs yet) ---
    pub huts: usize,
    pub barns: usize,
    pub lumber_yards: usize,
    pub stone_quarries: usize,
    pub farms: usize,

    // --- Workers (auto-assigned each tick / after production builds) ---
    pub workers_on_lumber_yards: usize,
    pub workers_on_stone_quarries: usize,
    pub workers_on_farms: usize,
}

impl Colony {
    // -------------------------------------------------------------------------
    // Worker math: slots, staffed buildings, free settlers
    // -------------------------------------------------------------------------

    /// Total worker slots across all lumber yards (each yard needs `lumber_yard_max_workers`).
    pub fn workers_needed_for_lumber_yards(&self, balance: &Balance) -> usize {
        self.lumber_yards * balance.buildings.lumber_yard_max_workers
    }

    /// Total worker slots across all stone quarries.
    pub fn workers_needed_for_stone_quarries(&self, balance: &Balance) -> usize {
        self.stone_quarries * balance.buildings.stone_quarry_max_workers
    }

    /// Total worker slots across all farms.
    pub fn workers_needed_for_farms(&self, balance: &Balance) -> usize {
        self.farms * balance.buildings.farm_max_workers
    }

    /// How many lumber yards have a **full** crew.
    ///
    /// Uses integer division: 3 workers with 2 per yard → 1 staffed yard (the 3rd worker
    /// is assigned but cannot complete a second yard — "all or nothing" per building).
    pub fn staffed_lumber_yards(&self, balance: &Balance) -> usize {
        self.workers_on_lumber_yards / balance.buildings.lumber_yard_max_workers
    }

    /// Fully staffed quarries (same division rule as lumber).
    pub fn staffed_stone_quarries(&self, balance: &Balance) -> usize {
        self.workers_on_stone_quarries / balance.buildings.stone_quarry_max_workers
    }

    /// Fully staffed farms.
    pub fn staffed_farms(&self, balance: &Balance) -> usize {
        self.workers_on_farms / balance.buildings.farm_max_workers
    }

    /// Settlers not assigned to production — only these count for `g *` yield.
    pub fn free_workers(&self) -> usize {
        let assigned =
            self.workers_on_lumber_yards + self.workers_on_stone_quarries + self.workers_on_farms;
        self.population.saturating_sub(assigned)
    }

    /// Total settlers currently working at production buildings.
    pub fn assigned_workers(&self) -> usize {
        self.workers_on_farms + self.workers_on_lumber_yards + self.workers_on_stone_quarries
    }

    /// Distribute settlers to production buildings.
    ///
    /// Algorithm:
    /// 1. `available = population - reserve_free_settlers`
    /// 2. Fill farm slots up to `farms × farm_max_workers`
    /// 3. Spend remainder on lumber, then quarries
    ///
    /// Does not log — caller logs when needed.
    pub(crate) fn auto_assign_workers(&mut self, balance: &Balance) {
        let mut available = self
            .population
            .saturating_sub(balance.population.reserve_free_settlers);

        let farm_slots = self.workers_needed_for_farms(balance);
        self.workers_on_farms = Self::assign_up_to(available, farm_slots);
        available -= self.workers_on_farms;

        let lumber_slots = self.workers_needed_for_lumber_yards(balance);
        self.workers_on_lumber_yards = Self::assign_up_to(available, lumber_slots);
        available -= self.workers_on_lumber_yards;

        let quarry_slots = self.workers_needed_for_stone_quarries(balance);
        self.workers_on_stone_quarries = Self::assign_up_to(available, quarry_slots);
    }

    /// `min(available settlers, open slots)` — never over-assign.
    fn assign_up_to(available: usize, slots: usize) -> usize {
        available.min(slots)
    }

    /// Set how many settlers work at one production type (`w farm 2` etc.).
    ///
    /// `count` is the **total** for that type, not a delta. Use `0` to unassign everyone there.
    pub fn set_workers(
        &mut self,
        site: WorkerSite,
        count: usize,
        balance: &Balance,
    ) -> Result<(), &'static str> {
        let max = match site {
            WorkerSite::Farm => self.workers_needed_for_farms(balance),
            WorkerSite::Lumber => self.workers_needed_for_lumber_yards(balance),
            WorkerSite::Quarry => self.workers_needed_for_stone_quarries(balance),
        };

        if count > max {
            return Err("Too many workers for that building type.");
        }

        let (farms, lumber, quarries) = match site {
            WorkerSite::Farm => (
                count,
                self.workers_on_lumber_yards,
                self.workers_on_stone_quarries,
            ),
            WorkerSite::Lumber => (self.workers_on_farms, count, self.workers_on_stone_quarries),
            WorkerSite::Quarry => (self.workers_on_farms, self.workers_on_lumber_yards, count),
        };

        if farms + lumber + quarries > self.population {
            return Err("Not enough settlers — lower another assignment or grow population.");
        }

        self.workers_on_farms = farms;
        self.workers_on_lumber_yards = lumber;
        self.workers_on_stone_quarries = quarries;
        Ok(())
    }

    /// Shrink assignments when buildings are lost or population drops.
    pub fn clamp_workers(&mut self, balance: &Balance) {
        self.workers_on_farms = self
            .workers_on_farms
            .min(self.workers_needed_for_farms(balance));
        self.workers_on_lumber_yards = self
            .workers_on_lumber_yards
            .min(self.workers_needed_for_lumber_yards(balance));
        self.workers_on_stone_quarries = self
            .workers_on_stone_quarries
            .min(self.workers_needed_for_stone_quarries(balance));

        while self.assigned_workers() > self.population {
            if self.workers_on_stone_quarries > 0 {
                self.workers_on_stone_quarries -= 1;
            } else if self.workers_on_lumber_yards > 0 {
                self.workers_on_lumber_yards -= 1;
            } else if self.workers_on_farms > 0 {
                self.workers_on_farms -= 1;
            } else {
                break;
            }
        }
    }

    /// Warnings when buildings exist but cannot run (not enough assigned workers).
    /// Used in logs after assignment changes or when building a production structure.
    pub fn understaffed_messages(&self, balance: &Balance) -> Vec<String> {
        let mut messages = Vec::new();

        if self.farms > 0 && self.workers_on_farms < self.workers_needed_for_farms(balance) {
            let idle = self.farms - self.staffed_farms(balance);
            messages.push(format!(
                "{idle} farm(s) idle: {}/{} workers assigned (need {} per farm)",
                self.workers_on_farms,
                self.workers_needed_for_farms(balance),
                balance.buildings.farm_max_workers
            ));
        }

        if self.lumber_yards > 0
            && self.workers_on_lumber_yards < self.workers_needed_for_lumber_yards(balance)
        {
            let idle = self.lumber_yards - self.staffed_lumber_yards(balance);
            messages.push(format!(
                "{idle} lumber yard(s) idle: {}/{} workers assigned (need {} per yard)",
                self.workers_on_lumber_yards,
                self.workers_needed_for_lumber_yards(balance),
                balance.buildings.lumber_yard_max_workers
            ));
        }

        if self.stone_quarries > 0
            && self.workers_on_stone_quarries < self.workers_needed_for_stone_quarries(balance)
        {
            let idle = self.stone_quarries - self.staffed_stone_quarries(balance);
            messages.push(format!(
                "{idle} quarry/quarries idle: {}/{} workers assigned (need {} per quarry)",
                self.workers_on_stone_quarries,
                self.workers_needed_for_stone_quarries(balance),
                balance.buildings.stone_quarry_max_workers
            ));
        }

        messages
    }

    // -------------------------------------------------------------------------
    // Active gathering (`g wood` / `g stone` / `g food`)
    // -------------------------------------------------------------------------

    /// Expected gather yield for `kind` today (UI preview). Zero if no free settlers.
    pub fn gather_yield(&self, kind: ResourceKind, balance: &Balance) -> usize {
        let free = self.free_workers();
        if free == 0 {
            return 0;
        }
        Self::yield_from_pop(
            balance.gather.base(kind),
            free,
            kind.gather_percent(),
        )
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

    pub fn gather_wood(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        self.gather(ResourceKind::Wood, balance)
    }

    pub fn gather_stone(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        self.gather(ResourceKind::Stone, balance)
    }

    pub fn gather_food(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        self.gather(ResourceKind::Food, balance)
    }

    // -------------------------------------------------------------------------
    // Construction (player spends resources; production buildings need workers later)
    // -------------------------------------------------------------------------

    /// +max population cap. Does not add settlers immediately — births fill huts over time.
    pub fn build_hut(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.buildings.build_hut_wood_cost {
            return Err("Not enough wood to build a hut!");
        }
        if self.stone < balance.buildings.build_hut_stone_cost {
            return Err("Not enough stone to build a hut!");
        }
        self.wood -= balance.buildings.build_hut_wood_cost;
        self.stone -= balance.buildings.build_hut_stone_cost;
        self.huts += 1;
        self.max_population += balance.buildings.hut_max_population_increase;
        Ok(1)
    }

    /// +food storage cap. Does not employ workers.
    pub fn build_barn(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.buildings.build_barn_wood_cost {
            return Err("Not enough wood to build a barn!");
        }
        if self.stone < balance.buildings.build_barn_stone_cost {
            return Err("Not enough stone to build a barn!");
        }
        self.wood -= balance.buildings.build_barn_wood_cost;
        self.stone -= balance.buildings.build_barn_stone_cost;
        self.barns += 1;
        self.max_food += balance.buildings.barn_max_food_storage_increase;
        Ok(1)
    }

    /// Adds a lumber yard. Passive wood only after auto-assign fills `lumber_yard_max_workers` per yard.
    pub fn build_lumber_yard(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.buildings.build_lumber_yard_wood_cost {
            return Err("Not enough wood to build a lumber yard!");
        }
        if self.stone < balance.buildings.build_lumber_yard_stone_cost {
            return Err("Not enough stone to build a lumber yard!");
        }
        self.wood -= balance.buildings.build_lumber_yard_wood_cost;
        self.stone -= balance.buildings.build_lumber_yard_stone_cost;
        self.lumber_yards += 1;
        Ok(1)
    }

    /// Adds a stone quarry. Passive stone when fully staffed.
    pub fn build_stone_quarry(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.buildings.build_stone_quarry_wood_cost {
            return Err("Not enough wood to build a stone quarry!");
        }
        if self.stone < balance.buildings.build_stone_quarry_stone_cost {
            return Err("Not enough stone to build a stone quarry!");
        }
        self.wood -= balance.buildings.build_stone_quarry_wood_cost;
        self.stone -= balance.buildings.build_stone_quarry_stone_cost;
        self.stone_quarries += 1;
        Ok(1)
    }

    /// Adds a farm. Passive food when fully staffed. Farms are filled first in auto-assign.
    pub fn build_farm(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.buildings.build_farm_wood_cost {
            return Err("Not enough wood to build a farm!");
        }
        if self.stone < balance.buildings.build_farm_stone_cost {
            return Err("Not enough stone to build a farm!");
        }
        self.wood -= balance.buildings.build_farm_wood_cost;
        self.stone -= balance.buildings.build_farm_stone_cost;
        self.farms += 1;
        Ok(1)
    }

    // -------------------------------------------------------------------------
    // Demolish (no resource refund; frees worker slots on next auto-assign)
    // -------------------------------------------------------------------------

    pub fn demolish_farm(&mut self) -> Result<(), &'static str> {
        if self.farms == 0 {
            return Err("No farms to demolish.");
        }
        self.farms -= 1;
        Ok(())
    }

    pub fn demolish_lumber_yard(&mut self) -> Result<(), &'static str> {
        if self.lumber_yards == 0 {
            return Err("No lumber yards to demolish.");
        }
        self.lumber_yards -= 1;
        Ok(())
    }

    pub fn demolish_stone_quarry(&mut self) -> Result<(), &'static str> {
        if self.stone_quarries == 0 {
            return Err("No stone quarries to demolish.");
        }
        self.stone_quarries -= 1;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Passive income (end of day, no action cost)
    // -------------------------------------------------------------------------

    /// Wood per tick from fully staffed lumber yards only.
    pub fn passive_wood(&self, balance: &Balance) -> usize {
        self.staffed_lumber_yards(balance) * balance.buildings.lumber_yard_wood_production
    }

    /// Stone per tick from fully staffed quarries.
    pub fn passive_stone(&self, balance: &Balance) -> usize {
        self.staffed_stone_quarries(balance) * balance.buildings.stone_quarry_stone_production
    }

    /// Food per tick from fully staffed farms.
    pub fn passive_food(&self, balance: &Balance) -> usize {
        self.staffed_farms(balance) * balance.buildings.farm_food_production
    }

    /// Add passive resources to colony stockpiles.
    ///
    /// Food is clipped to `max_food` here; any overflow is handled by spoilage logic in `tick`.
    pub fn apply_passive_income(&mut self, balance: &Balance) -> (usize, usize, usize) {
        let wood = self.passive_wood(balance);
        let stone = self.passive_stone(balance);

        let food_gain = self.passive_food(balance);
        let food = food_gain.min(self.max_food.saturating_sub(self.food));
        self.food += food;
        self.wood += wood;
        self.stone += stone;
        (wood, stone, food)
    }

    /// Active gather yield: `base + free × percent / 100` (integer division).
    pub fn yield_from_pop(base: usize, pop: usize, percent: usize) -> usize {
        base + pop * percent / 100
    }
}

impl Default for Colony {
    /// Starting colony: 5 settlers in 1 hut, modest resources, no production buildings.
    fn default() -> Self {
        Self {
            name: "Default colony".to_string(),
            wood: 50,
            stone: 30,
            food: 20,
            population: 5,
            max_population: 5,
            max_food: 25,
            huts: 1,
            barns: 0,
            lumber_yards: 0,
            stone_quarries: 0,
            farms: 0,
            workers_on_lumber_yards: 0,
            workers_on_stone_quarries: 0,
            workers_on_farms: 0,
        }
    }
}
