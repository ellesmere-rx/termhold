//! Core simulation: colony state, balance constants, day tick, commands.
//!
//! # Game loop (see `main.rs`)
//!
//! Each iteration of the main loop is one in-game day:
//!
//! 1. **Render** — show colony state (`ui::render`).
//! 2. **Command** — player picks one action (`Game::process_command`).
//! 3. **Tick** — end-of-day simulation (`Game::tick`): food, births, workers, passive income.
//!
//! Invalid or empty input still advances the day (tick always runs).
//!
//! # Two ways to earn resources
//!
//! 1. **Active gathering** (`g wood` / `g stone` / `g food`) — the player's command for that day.
//!    Yield scales with **free settlers** (not total population): see [`yield_from_pop`].
//!    Fails if `free_workers() == 0` (everyone is assigned to buildings).
//!
//! 2. **Passive production** — applied automatically at the end of [`Game::tick`], after workers
//!    are assigned. Only **fully staffed** production buildings count:
//!    `staffed = workers_on_type / workers_per_building` (integer division).
//!    A yard with 1 of 2 required workers produces nothing ("all or nothing" rule).
//!
//! # Worker model (auto-assign)
//!
//! Settlers are split into **assigned** (on farms / lumber yards / quarries) and **free**.
//! Assigned settlers do not boost active gathering.
//!
//! [`Colony::auto_assign_workers`] runs:
//! - at the end of every [`Game::tick`] (after population may have changed);
//! - at the end of every [`Game::process_command`] (e.g. after building a farm).
//!
//! Assignment order (when settlers are scarce):
//! **farms → lumber yards → stone quarries**.
//!
//! Optional: [`Balance::reserve_free_settlers`] keeps N settlers out of auto-assign
//! so gathering stays possible even with many buildings.
//!
//! Hut and barn do not employ anyone — they only raise caps (population / food storage).
//!
//! # Demolish production buildings
//!
//! `d farm` / `d lumber` / `d quarry` removes one building (no resource refund).
//! Settlers are re-assigned immediately — use this when everyone is assigned and
//! gathering is blocked.
//!
//! # Win / lose
//!
//! - Lose: `population == 0` (starvation).
//! - Survive to day 180: run ends (victory timer).

use rand::RngExt;

/// Maximum log lines kept in memory; oldest are dropped.
const MAX_LOG_SIZE: usize = 100;

/// Player actions parsed from CLI input in `ui.rs`.
#[derive(PartialEq, Debug)]
pub enum Commands {
    GetWood,
    GetStone,
    GetFood,
    BuildHut,
    BuildLumberYard,
    BuildStoneQuarry,
    BuildFarm,
    BuildBarn,
    DemolishFarm,
    DemolishLumberYard,
    DemolishStoneQuarry,
    Quit,
}

/// Global time — only `days` for now. Incremented at the very end of each [`Game::tick`].
pub struct World {
    pub days: usize,
}

impl World {}

impl Default for World {
    fn default() -> Self {
        Self { days: 1 }
    }
}

/// All mutable colony state: resources, caps, building counts, worker assignments.
///
/// Worker fields (`workers_on_*`) are written only by [`Colony::auto_assign_workers`];
/// they are a cache of the last auto-assignment, not manual player input.
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
        self.lumber_yards * balance.lumber_yard_max_workers
    }

    /// Total worker slots across all stone quarries.
    pub fn workers_needed_for_stone_quarries(&self, balance: &Balance) -> usize {
        self.stone_quarries * balance.stone_quarry_max_workers
    }

    /// Total worker slots across all farms.
    pub fn workers_needed_for_farms(&self, balance: &Balance) -> usize {
        self.farms * balance.farm_max_workers
    }

    /// How many lumber yards have a **full** crew.
    ///
    /// Uses integer division: 3 workers with 2 per yard → 1 staffed yard (the 3rd worker
    /// is assigned but cannot complete a second yard — "all or nothing" per building).
    pub fn staffed_lumber_yards(&self, balance: &Balance) -> usize {
        self.workers_on_lumber_yards / balance.lumber_yard_max_workers
    }

    /// Fully staffed quarries (same division rule as lumber).
    pub fn staffed_stone_quarries(&self, balance: &Balance) -> usize {
        self.workers_on_stone_quarries / balance.stone_quarry_max_workers
    }

    /// Fully staffed farms.
    pub fn staffed_farms(&self, balance: &Balance) -> usize {
        self.workers_on_farms / balance.farm_max_workers
    }

    /// Settlers not assigned to production — only these count for `g *` yield.
    pub fn free_workers(&self) -> usize {
        let assigned =
            self.workers_on_lumber_yards + self.workers_on_stone_quarries + self.workers_on_farms;
        self.population.saturating_sub(assigned)
    }

    /// Total settlers currently working at production buildings.
    pub fn assigned_workers(&self) -> usize {
        self.population - self.free_workers()
    }

    /// Distribute settlers to production buildings.
    ///
    /// Algorithm:
    /// 1. `available = population - reserve_free_settlers`
    /// 2. Fill farm slots up to `farms × farm_max_workers`
    /// 3. Spend remainder on lumber, then quarries
    ///
    /// Does not log — [`Game::refresh_worker_assignments`] handles logging.
    fn auto_assign_workers(&mut self, balance: &Balance) {
        let mut available = self
            .population
            .saturating_sub(balance.reserve_free_settlers);

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
                balance.farm_max_workers
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
                balance.lumber_yard_max_workers
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
                balance.stone_quarry_max_workers
            ));
        }

        messages
    }

    // -------------------------------------------------------------------------
    // Active gathering (`g wood` / `g stone` / `g food`)
    // -------------------------------------------------------------------------

    /// Expected wood from `g wood` today (UI preview). Zero if no free settlers.
    pub fn wood_yield(&self, balance: &Balance) -> usize {
        let free = self.free_workers();
        if free == 0 {
            return 0;
        }
        yield_from_pop(balance.gather_wood_base, free, 40, 1)
    }

    /// Expected stone from `g stone`. Uses 33% per free settler.
    pub fn stone_yield(&self, balance: &Balance) -> usize {
        let free = self.free_workers();
        if free == 0 {
            return 0;
        }
        yield_from_pop(balance.gather_stone_base, free, 33, 1)
    }

    /// Expected food from `g food`. Uses 50% per free settler.
    pub fn food_yield(&self, balance: &Balance) -> usize {
        let free = self.free_workers();
        if free == 0 {
            return 0;
        }
        yield_from_pop(balance.gather_food_base, free, 50, 1)
    }

    pub fn gather_wood(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.free_workers() == 0 {
            return Err(
                "No free settlers — everyone works at buildings. Grow population or leave fewer workers assigned.",
            );
        }
        let gather_yield = self.wood_yield(balance);
        self.wood += gather_yield;
        Ok(gather_yield)
    }

    pub fn gather_stone(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.free_workers() == 0 {
            return Err(
                "No free settlers — everyone works at buildings. Grow population or leave fewer workers assigned.",
            );
        }
        let gather_yield = self.stone_yield(balance);
        self.stone += gather_yield;
        Ok(gather_yield)
    }

    pub fn gather_food(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.free_workers() == 0 {
            return Err(
                "No free settlers — everyone works at buildings. Grow population or leave fewer workers assigned.",
            );
        }
        let gather_yield = self
            .food_yield(balance)
            .min(self.max_food.saturating_sub(self.food));
        self.food += gather_yield;
        Ok(gather_yield)
    }

    // -------------------------------------------------------------------------
    // Construction (player spends resources; production buildings need workers later)
    // -------------------------------------------------------------------------

    /// +max population cap. Does not add settlers immediately — births fill huts over time.
    pub fn build_hut(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.build_hut_wood_cost {
            return Err("Not enough wood to build a hut!");
        }
        if self.stone < balance.build_hut_stone_cost {
            return Err("Not enough stone to build a hut!");
        }
        self.wood -= balance.build_hut_wood_cost;
        self.stone -= balance.build_hut_stone_cost;
        self.huts += 1;
        self.max_population += balance.hut_max_population_increase;
        Ok(1)
    }

    /// +food storage cap. Does not employ workers.
    pub fn build_barn(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.build_barn_wood_cost {
            return Err("Not enough wood to build a barn!");
        }
        if self.stone < balance.build_barn_stone_cost {
            return Err("Not enough stone to build a barn!");
        }
        self.wood -= balance.build_barn_wood_cost;
        self.stone -= balance.build_barn_stone_cost;
        self.barns += 1;
        self.max_food += balance.barn_max_food_storage_increase;
        Ok(1)
    }

    /// Adds a lumber yard. Passive wood only after auto-assign fills `lumber_yard_max_workers` per yard.
    pub fn build_lumber_yard(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.build_lumber_yard_wood_cost {
            return Err("Not enough wood to build a lumber yard!");
        }
        if self.stone < balance.build_lumber_yard_stone_cost {
            return Err("Not enough stone to build a lumber yard!");
        }
        self.wood -= balance.build_lumber_yard_wood_cost;
        self.stone -= balance.build_lumber_yard_stone_cost;
        self.lumber_yards += 1;
        Ok(1)
    }

    /// Adds a stone quarry. Passive stone when fully staffed.
    pub fn build_stone_quarry(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.build_stone_quarry_wood_cost {
            return Err("Not enough wood to build a stone quarry!");
        }
        if self.stone < balance.build_stone_quarry_stone_cost {
            return Err("Not enough stone to build a stone quarry!");
        }
        self.wood -= balance.build_stone_quarry_wood_cost;
        self.stone -= balance.build_stone_quarry_stone_cost;
        self.stone_quarries += 1;
        Ok(1)
    }

    /// Adds a farm. Passive food when fully staffed. Farms are filled first in auto-assign.
    pub fn build_farm(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.wood < balance.build_farm_wood_cost {
            return Err("Not enough wood to build a farm!");
        }
        if self.stone < balance.build_farm_stone_cost {
            return Err("Not enough stone to build a farm!");
        }
        self.wood -= balance.build_farm_wood_cost;
        self.stone -= balance.build_farm_stone_cost;
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
        self.staffed_lumber_yards(balance) * balance.lumber_yard_wood_production
    }

    /// Stone per tick from fully staffed quarries.
    pub fn passive_stone(&self, balance: &Balance) -> usize {
        self.staffed_stone_quarries(balance) * balance.stone_quarry_stone_production
    }

    /// Food per tick from fully staffed farms.
    pub fn passive_food(&self, balance: &Balance) -> usize {
        self.staffed_farms(balance) * balance.farm_food_production
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
}

impl Default for Colony {
    /// Starting colony: 5 settlers in 1 hut, modest resources, no production buildings.
    fn default() -> Self {
        Self {
            name: "Default colony".to_string(),
            wood: 50,
            stone: 30,
            food: 25,
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

/// Root game state: colony + calendar + event log + balance sheet.
pub struct Game {
    pub colony: Colony,
    pub world: World,
    /// Newest entries at the end. Prefixed with current day in [`Game::logs`].
    pub logs: Vec<String>,
    pub balance: Balance,
    pub gameover: bool,
}

impl Game {
    /// Re-run [`Colony::auto_assign_workers`] and write logs when assignment changes.
    ///
    /// `log_if_understaffed`: also log idle buildings (e.g. right after `b farm` when
    /// there are not enough settlers to staff everything).
    fn refresh_worker_assignments(&mut self, log_if_understaffed: bool) {
        let farms_before = self.colony.workers_on_farms;
        let lumber_before = self.colony.workers_on_lumber_yards;
        let quarries_before = self.colony.workers_on_stone_quarries;

        self.colony.auto_assign_workers(&self.balance);

        let changed = farms_before != self.colony.workers_on_farms
            || lumber_before != self.colony.workers_on_lumber_yards
            || quarries_before != self.colony.workers_on_stone_quarries;

        if changed {
            self.logs(format!(
                "Workers: farms {}, lumber {}, quarries {} ({} free for gathering)",
                self.colony.workers_on_farms,
                self.colony.workers_on_lumber_yards,
                self.colony.workers_on_stone_quarries,
                self.colony.free_workers(),
            ));
        }

        if changed || log_if_understaffed {
            for msg in self.colony.understaffed_messages(&self.balance) {
                self.logs(msg);
            }
        }
    }

    /// Farm, lumber yard, and quarry add worker slots — worth logging understaffing after build.
    fn should_refresh_workers_with_understaffed_log(command: &Commands) -> bool {
        matches!(
            command,
            Commands::BuildLumberYard
                | Commands::BuildStoneQuarry
                | Commands::BuildFarm
                | Commands::DemolishLumberYard
                | Commands::DemolishStoneQuarry
                | Commands::DemolishFarm
        )
    }

    /// End-of-day simulation. Order matters:
    ///
    /// 1. Check lose/win by population / day 365
    /// 2. Food upkeep (1 per settler) or starvation (−1 pop)
    /// 3. Random birth if food and housing allow
    /// 4. Spoil food above `max_food`
    /// 5. Auto-assign workers → passive income from staffed buildings
    /// 6. Advance day counter
    pub fn tick(&mut self) {
        // --- Lose / win ---
        if self.colony.population == 0 {
            println!("Gameover. Colony is dead.");
            self.gameover = true;
        } else if self.world.days == 365 {
            println!("Gameover. Colony reached 365 days.");
            self.gameover = true;
        }

        // --- Food: upkeep or starvation ---
        if self.colony.food < self.colony.population {
            self.colony.food = 0;
            self.logs("Not enough food! Colony is starving, population is decreasing (-1)".into());
            self.colony.population = self.colony.population.saturating_sub(1);
        } else {
            self.colony.food -= self.colony.population;
            self.logs(format!("Colony consumes {} food", self.colony.population));

            // --- Birth: one roll per day if food buffer and hut space exist ---
            if self.colony.population < self.colony.max_population {
                let min_food = self.colony.population + self.balance.population_increase_cost;
                if self.colony.food >= min_food {
                    let chance = self.balance.birth_chance_percent;
                    let mut rng = rand::rng();
                    let roll: u8 = rng.random_range(0..100);
                    if roll < chance {
                        self.colony.food = self
                            .colony
                            .food
                            .saturating_sub(self.balance.population_increase_cost);
                        self.colony.population += 1;
                        self.logs(format!(
                            "Birth! population +1 (chance {chance}%), food -{}",
                            self.balance.population_increase_cost
                        ));
                    }
                }
            }
        }

        // --- Storage cap: food above max_food is destroyed ---
        if self.colony.food > self.colony.max_food {
            let spoiled = self.colony.food - self.colony.max_food;
            self.colony.food = self.colony.max_food;
            self.logs(format!(
                "Spoiled {spoiled} food (storage {}/{})",
                self.colony.food, self.colony.max_food
            ));
        }

        // --- Workers + passive payout (after pop may have changed) ---
        self.refresh_worker_assignments(false);

        self.colony.apply_passive_income(&self.balance);

        self.world.days += 1;
    }

    /// Apply the player's one action for this day, then re-assign workers.
    ///
    /// Gather/build errors are logged but do not skip the upcoming `tick`.
    pub fn process_command(&mut self, command: Commands) {
        let log_understaffed = Self::should_refresh_workers_with_understaffed_log(&command);

        match command {
            Commands::GetWood => match self.colony.gather_wood(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Gathered wood (+{gain}) with {} free settler(s)",
                    self.colony.free_workers()
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::GetStone => match self.colony.gather_stone(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Gathered stone (+{gain}) with {} free settler(s)",
                    self.colony.free_workers()
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::GetFood => match self.colony.gather_food(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Gathered food (+{gain}) with {} free settler(s)",
                    self.colony.free_workers()
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildHut => match self.colony.build_hut(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Huts (+{gain}), max pop +{}, spent {} wood, spent {} stone",
                    self.balance.hut_max_population_increase,
                    self.balance.build_hut_wood_cost,
                    self.balance.build_hut_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildLumberYard => match self.colony.build_lumber_yard(&self.balance) {
                Ok(_) => self.logs(format!(
                    "Lumber yard built (+{} wood/day when {} workers assigned), spent {} wood, {} stone",
                    self.balance.lumber_yard_wood_production,
                    self.balance.lumber_yard_max_workers,
                    self.balance.build_lumber_yard_wood_cost,
                    self.balance.build_lumber_yard_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildStoneQuarry => match self.colony.build_stone_quarry(&self.balance) {
                Ok(_) => self.logs(format!(
                    "Stone quarry built (+{} stone/day when {} workers assigned), spent {} wood, {} stone",
                    self.balance.stone_quarry_stone_production,
                    self.balance.stone_quarry_max_workers,
                    self.balance.build_stone_quarry_wood_cost,
                    self.balance.build_stone_quarry_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildBarn => match self.colony.build_barn(&self.balance) {
                Ok(_) => self.logs(format!(
                    "Barn built (+{} food storage, max {}), spent {} wood, spent {} stone",
                    self.balance.barn_max_food_storage_increase,
                    self.colony.max_food,
                    self.balance.build_barn_wood_cost,
                    self.balance.build_barn_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildFarm => match self.colony.build_farm(&self.balance) {
                Ok(_) => self.logs(format!(
                    "Farm built (+{} food/day when {} workers assigned), spent {} wood, {} stone",
                    self.balance.farm_food_production,
                    self.balance.farm_max_workers,
                    self.balance.build_farm_wood_cost,
                    self.balance.build_farm_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },

            Commands::DemolishFarm => match self.colony.demolish_farm() {
                Ok(()) => self.logs(format!(
                    "Farm demolished ({} remaining). Settlers reassigned.",
                    self.colony.farms
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::DemolishLumberYard => match self.colony.demolish_lumber_yard() {
                Ok(()) => self.logs(format!(
                    "Lumber yard demolished ({} remaining). Settlers reassigned.",
                    self.colony.lumber_yards
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::DemolishStoneQuarry => match self.colony.demolish_stone_quarry() {
                Ok(()) => self.logs(format!(
                    "Stone quarry demolished ({} remaining). Settlers reassigned.",
                    self.colony.stone_quarries
                )),
                Err(msg) => self.logs(msg.to_string()),
            },

            Commands::Quit => {}
        }

        self.refresh_worker_assignments(log_understaffed);
    }

    /// Append a line to the event log. Drops oldest entries beyond [`MAX_LOG_SIZE`].
    pub fn logs(&mut self, text: String) {
        self.logs
            .push(format!("Day {} | {}", self.world.days, text));

        let extra = self.logs.len().saturating_sub(MAX_LOG_SIZE);
        self.logs.drain(..extra);
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            colony: Colony::default(),
            world: World::default(),
            logs: Vec::with_capacity(100),
            balance: Balance::default(),
            gameover: false,
        }
    }
}

/// Active gather yield: `base + max(pop × percent/100, min_bonus)`.
///
/// `pop` is **free settlers**, not total colony population.
///
/// Percent per resource: wood 40%, stone 33%, food 50%.
/// `min_bonus` guarantees at least +1 from pop when `pop > 0` (integer `%` can round down to 0).
///
/// Callers must pass `pop == 0` only when gather is blocked — otherwise UI would show a false yield.
fn yield_from_pop(base: usize, pop: usize, percent: usize, min_bonus: usize) -> usize {
    let bonus = (pop * percent / 100).max(min_bonus);
    base + bonus
}

// =============================================================================
// Tests — worker auto-assign is heavily covered; see `assert_workers` helper.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn balance() -> Balance {
        Balance::default()
    }

    fn colony(pop: usize, farms: usize, lumber: usize, quarries: usize) -> Colony {
        let mut colony = Colony::default();
        colony.population = pop;
        colony.farms = farms;
        colony.lumber_yards = lumber;
        colony.stone_quarries = quarries;
        colony
    }

    fn assign(colony: &mut Colony, balance: &Balance) {
        colony.auto_assign_workers(balance);
    }

    /// Assert worker counts and the invariant: assigned + free == population.
    fn assert_workers(
        colony: &Colony,
        balance: &Balance,
        farms: usize,
        lumber: usize,
        quarries: usize,
        free: usize,
    ) {
        assert_eq!(colony.workers_on_farms, farms, "workers_on_farms");
        assert_eq!(
            colony.workers_on_lumber_yards, lumber,
            "workers_on_lumber_yards"
        );
        assert_eq!(
            colony.workers_on_stone_quarries, quarries,
            "workers_on_stone_quarries"
        );
        assert_eq!(colony.free_workers(), free, "free_workers");
        assert_eq!(
            colony.assigned_workers() + colony.free_workers(),
            colony.population,
            "assigned + free must equal population"
        );
        assert!(
            colony.workers_on_farms <= colony.workers_needed_for_farms(balance),
            "farm workers exceed slots"
        );
        assert!(
            colony.workers_on_lumber_yards <= colony.workers_needed_for_lumber_yards(balance),
            "lumber workers exceed slots"
        );
        assert!(
            colony.workers_on_stone_quarries <= colony.workers_needed_for_stone_quarries(balance),
            "quarry workers exceed slots"
        );
    }

    // --- yield_from_pop ---

    #[test]
    fn test_yeild_from_pop() {
        let result = yield_from_pop(5, 2, 40, 1);
        assert_eq!(result, 6);
    }

    // --- auto_assign: no buildings ---

    #[test]
    fn auto_assign_no_buildings_leaves_everyone_free() {
        let balance = balance();
        let mut colony = colony(5, 0, 0, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 0, 0, 0, 5);
    }

    #[test]
    fn auto_assign_zero_population_assigns_nobody() {
        let balance = balance();
        let mut colony = colony(0, 2, 1, 1);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 0, 0, 0, 0);
    }

    // --- auto_assign: priority farm → lumber → quarry ---

    #[test]
    fn auto_assign_farms_before_lumber_when_pop_is_short() {
        let balance = balance();
        // 1 farm needs 2, 1 lumber needs 2, only 3 settlers → farm full, lumber gets 1
        let mut colony = colony(3, 1, 1, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 2, 1, 0, 0);
        assert_eq!(colony.staffed_farms(&balance), 1);
        assert_eq!(colony.staffed_lumber_yards(&balance), 0);
    }

    #[test]
    fn auto_assign_lumber_before_quarries_when_pop_is_short() {
        let balance = balance();
        // farm 2 + lumber 2 + quarry needs 2, only 5 settlers → quarry gets 1
        let mut colony = colony(5, 1, 1, 1);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 2, 2, 1, 0);
        assert_eq!(colony.staffed_stone_quarries(&balance), 0);
    }

    #[test]
    fn auto_assign_fills_all_types_when_pop_allows() {
        let balance = balance();
        let mut colony = colony(7, 1, 1, 1);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 2, 2, 2, 1);
        assert_eq!(colony.staffed_farms(&balance), 1);
        assert_eq!(colony.staffed_lumber_yards(&balance), 1);
        assert_eq!(colony.staffed_stone_quarries(&balance), 1);
    }

    // --- auto_assign: multiple buildings of one type ---

    #[test]
    fn auto_assign_multiple_farms_fill_before_lumber() {
        let balance = balance();
        // 2 farms need 4, 1 lumber needs 2, 5 settlers → farms 4, lumber 1
        let mut colony = colony(5, 2, 1, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 4, 1, 0, 0);
        assert_eq!(colony.staffed_farms(&balance), 2);
        assert_eq!(colony.staffed_lumber_yards(&balance), 0);
    }

    #[test]
    fn auto_assign_multiple_lumber_yards_partially_staffed() {
        let balance = balance();
        // 2 yards need 4 workers, only 3 settlers
        let mut colony = colony(3, 0, 2, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 0, 3, 0, 0);
        assert_eq!(colony.staffed_lumber_yards(&balance), 1);
    }

    // --- auto_assign: reserve & caps ---

    #[test]
    fn auto_assign_reserve_keeps_settlers_free() {
        let balance = Balance {
            reserve_free_settlers: 1,
            ..balance()
        };
        let mut colony = colony(4, 2, 0, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 3, 0, 0, 1);
    }

    #[test]
    fn auto_assign_reserve_larger_than_population_assigns_nobody() {
        let balance = Balance {
            reserve_free_settlers: 10,
            ..balance()
        };
        let mut colony = colony(5, 1, 1, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 0, 0, 0, 5);
    }

    #[test]
    fn auto_assign_never_assigns_more_than_population() {
        let balance = balance();
        let mut colony = colony(3, 5, 5, 5);
        assign(&mut colony, &balance);
        assert_eq!(colony.assigned_workers(), 3);
        assert_eq!(colony.free_workers(), 0);
    }

    // --- auto_assign: stability ---

    #[test]
    fn auto_assign_is_idempotent() {
        let balance = balance();
        let mut colony = colony(6, 1, 2, 1);
        assign(&mut colony, &balance);
        let first = (
            colony.workers_on_farms,
            colony.workers_on_lumber_yards,
            colony.workers_on_stone_quarries,
        );
        assign(&mut colony, &balance);
        assert_eq!(
            (
                colony.workers_on_farms,
                colony.workers_on_lumber_yards,
                colony.workers_on_stone_quarries
            ),
            first
        );
    }

    #[test]
    fn auto_assign_updates_when_population_grows() {
        let balance = balance();
        let mut colony = colony(4, 2, 0, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 4, 0, 0, 0);

        colony.population = 5;
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 4, 0, 0, 1);
    }

    // --- auto_assign: links to production & gather ---

    #[test]
    fn auto_assign_staffed_buildings_produce_passive_income() {
        let balance = balance();
        let mut colony = colony(8, 1, 2, 1);
        colony.food = 0;
        assign(&mut colony, &balance);

        let (wood, stone, food) = colony.apply_passive_income(&balance);
        assert_eq!(wood, 6);
        assert_eq!(stone, 2);
        assert_eq!(food, 2);
    }

    #[test]
    fn auto_assign_partial_crew_produces_no_passive() {
        let balance = balance();
        let mut colony = colony(1, 0, 1, 0);
        assign(&mut colony, &balance);
        assert_eq!(colony.passive_wood(&balance), 0);
    }

    #[test]
    fn auto_assign_free_settlers_drive_gather_yield() {
        let balance = balance();
        let mut colony = colony(5, 1, 0, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 2, 0, 0, 3);
        assert_eq!(
            colony.wood_yield(&balance),
            yield_from_pop(balance.gather_wood_base, 3, 40, 1)
        );
    }

    #[test]
    fn auto_assign_all_busy_blocks_gather() {
        let balance = balance();
        let mut colony = colony(4, 2, 0, 0);
        assign(&mut colony, &balance);
        assert_workers(&colony, &balance, 4, 0, 0, 0);
        assert_eq!(colony.wood_yield(&balance), 0);
        assert!(colony.gather_wood(&balance).is_err());
    }

    // --- auto_assign: understaffed messages ---

    #[test]
    fn auto_assign_understaffed_messages_list_idle_buildings() {
        let balance = balance();
        let mut colony = colony(3, 0, 2, 0);
        assign(&mut colony, &balance);

        let messages = colony.understaffed_messages(&balance);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("lumber yard"));
        assert!(messages[0].contains("idle"));
    }

    #[test]
    fn auto_assign_fully_staffed_has_no_understaffed_messages() {
        let balance = balance();
        let mut colony = colony(2, 1, 0, 0);
        assign(&mut colony, &balance);
        assert!(colony.understaffed_messages(&balance).is_empty());
    }

    // --- Game integration: refresh_worker_assignments via tick / commands ---

    #[test]
    fn game_build_farm_auto_assigns_workers() {
        let mut game = Game::default();
        game.colony.population = 5;

        game.process_command(Commands::BuildFarm);

        assert_eq!(game.colony.workers_on_farms, 2);
        assert_eq!(game.colony.free_workers(), 3);
    }

    #[test]
    fn game_build_production_logs_worker_assignment() {
        let mut game = Game::default();
        game.colony.population = 5;
        game.colony.stone = 100;

        game.process_command(Commands::BuildLumberYard);

        assert!(
            game.logs.iter().any(|log| log.contains("Workers:")),
            "expected worker log after building lumber yard, got: {:?}",
            game.logs
        );
    }

    #[test]
    fn game_tick_reassigns_workers_after_starvation() {
        let mut game = Game::default();
        game.colony.population = 4;
        game.colony.farms = 2;
        game.colony.food = 0;
        game.colony.workers_on_farms = 0;

        game.tick();

        assert_eq!(game.colony.population, 3);
        assert_eq!(game.colony.workers_on_farms, 3);
        assert_eq!(game.colony.free_workers(), 0);
        assert_eq!(game.colony.staffed_farms(&game.balance), 1);
    }

    #[test]
    fn game_tick_does_not_duplicate_worker_log_when_unchanged() {
        let mut game = Game::default();
        game.colony.population = 5;
        game.colony.food = 100;
        assign(&mut game.colony, &game.balance);
        let logs_before = game.logs.len();

        game.tick();

        let new_worker_logs = game.logs[logs_before..]
            .iter()
            .filter(|log| log.contains("Workers:"))
            .count();
        assert_eq!(new_worker_logs, 0);
    }

    #[test]
    fn demolish_farm_frees_settlers_for_gathering() {
        let balance = balance();
        let mut colony = colony(4, 2, 0, 0);
        assign(&mut colony, &balance);
        assert_eq!(colony.free_workers(), 0);

        colony.demolish_farm().unwrap();
        assign(&mut colony, &balance);

        assert_eq!(colony.farms, 1);
        assert_eq!(colony.workers_on_farms, 2);
        assert_eq!(colony.free_workers(), 2);
        assert!(colony.gather_wood(&balance).is_ok());
    }

    #[test]
    fn demolish_fails_when_no_buildings() {
        let mut colony = colony(5, 0, 0, 0);
        assert!(colony.demolish_farm().is_err());
        assert!(colony.demolish_lumber_yard().is_err());
        assert!(colony.demolish_stone_quarry().is_err());
    }

    #[test]
    fn game_demolish_reassigns_workers() {
        let mut game = Game::default();
        game.colony.population = 4;
        game.colony.farms = 2;
        assign(&mut game.colony, &game.balance);
        assert_eq!(game.colony.free_workers(), 0);

        game.process_command(Commands::DemolishFarm);

        assert_eq!(game.colony.farms, 1);
        assert_eq!(game.colony.free_workers(), 2);
        assert!(
            game.logs.iter().any(|log| log.contains("Farm demolished")),
            "got: {:?}",
            game.logs
        );
    }
}
