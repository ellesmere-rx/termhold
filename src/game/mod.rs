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
//! # Workers
//!
//! Assignments change only when **you** run a `w` command — nothing runs automatically
//! at end of day. `w` commands do **not** advance the day (see `main.rs`).
//!
//! - `w farm 2` / `w lumber 0` / `w quarry 1` — set exact counts (`0` = unassign)
//! - `w auto` — one-shot auto-fill (farm → lumber → quarry priority)
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
mod balance;
mod colony;
mod commands;
mod world;

pub use balance::Balance;
pub use colony::Colony;
pub use commands::{Commands, WorkerSite};
pub use world::World;

use rand::RngExt;

/// Maximum log lines kept in memory; oldest are dropped.
const MAX_LOG_SIZE: usize = 100;
/// Last day — survive to this day to win.
pub const WIN_DAY: usize = 180;

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
    fn log_worker_assignment(&mut self) {
        self.logs(format!(
            "Workers: farms {}, lumber {}, quarries {} ({} free for gathering)",
            self.colony.workers_on_farms,
            self.colony.workers_on_lumber_yards,
            self.colony.workers_on_stone_quarries,
            self.colony.free_workers(),
        ));
    }

    /// One-shot auto-assign (`w auto` only). Logs result and understaffing.
    fn run_auto_assign_workers(&mut self) {
        self.colony.auto_assign_workers(&self.balance);
        self.log_worker_assignment();
        for msg in self.colony.understaffed_messages(&self.balance) {
            self.logs(msg);
        }
    }

    fn clamp_workers_after_build_change(&mut self) {
        self.colony.clamp_workers(&self.balance);
        for msg in self.colony.understaffed_messages(&self.balance) {
            self.logs(msg);
        }
    }

    fn worker_site_label(site: WorkerSite) -> &'static str {
        match site {
            WorkerSite::Farm => "farm",
            WorkerSite::Lumber => "lumber",
            WorkerSite::Quarry => "quarry",
        }
    }

    /// Production build/demolish may require trimming worker counts.
    fn should_clamp_workers_after_command(command: &Commands) -> bool {
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
    /// 1. Check lose/win by population / day [`WIN_DAY`]
    /// 2. Food upkeep (1 per settler) or starvation (−1 pop)
    /// 3. Random birth if food and housing allow
    /// 4. Spoil food above `max_food`
    /// 5. Clamp worker counts if pop/buildings changed → passive income
    /// 6. Advance day counter
    pub fn tick(&mut self) {
        // --- Lose / win ---
        if self.colony.population == 0 {
            println!("Gameover. Colony is dead.");
            self.gameover = true;
        } else if self.world.days >= WIN_DAY {
            println!("Victory! Colony survived {WIN_DAY} days.");
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

        // --- Clamp workers if pop dropped, then passive payout ---
        self.colony.clamp_workers(&self.balance);

        self.colony.apply_passive_income(&self.balance);

        self.world.days += 1;
    }

    /// Apply the player's one action for this day, then re-assign workers.
    ///
    /// Gather/build errors are logged but do not skip the upcoming `tick`.
    pub fn process_command(&mut self, command: Commands) {
        let clamp_after = Self::should_clamp_workers_after_command(&command);

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
                    "Farm demolished ({} remaining).",
                    self.colony.farms
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::DemolishLumberYard => match self.colony.demolish_lumber_yard() {
                Ok(()) => self.logs(format!(
                    "Lumber yard demolished ({} remaining).",
                    self.colony.lumber_yards
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::DemolishStoneQuarry => match self.colony.demolish_stone_quarry() {
                Ok(()) => self.logs(format!(
                    "Stone quarry demolished ({} remaining).",
                    self.colony.stone_quarries
                )),
                Err(msg) => self.logs(msg.to_string()),
            },

            Commands::SetWorkers { site, count } => {
                match self.colony.set_workers(site, count, &self.balance) {
                    Ok(()) => {
                        self.logs(format!(
                            "Set {} workers to {}.",
                            Self::worker_site_label(site),
                            count
                        ));
                        self.log_worker_assignment();
                        for msg in self.colony.understaffed_messages(&self.balance) {
                            self.logs(msg);
                        }
                    }
                    Err(msg) => self.logs(msg.to_string()),
                }
            }
            Commands::WorkersAuto => {
                self.logs("Auto-assign (farm → lumber → quarry).".into());
                self.run_auto_assign_workers();
            }

            Commands::Quit => {}
        }

        if clamp_after {
            self.clamp_workers_after_build_change();
        }
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
        assert_eq!(Colony::yield_from_pop(5, 2, 40), 5);
        assert_eq!(Colony::yield_from_pop(5, 5, 40), 7);
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
            Colony::yield_from_pop(balance.gather_wood_base, 3, 40)
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

    // --- Game integration: tick clamps only, w auto assigns ---

    #[test]
    fn game_build_farm_does_not_auto_assign_workers() {
        let mut game = Game::default();
        game.colony.population = 5;

        game.process_command(Commands::BuildFarm);

        assert_eq!(game.colony.workers_on_farms, 0);
        assert_eq!(game.colony.free_workers(), 5);
    }

    #[test]
    fn game_w_auto_assigns_workers() {
        let mut game = Game::default();
        game.colony.population = 5;

        game.process_command(Commands::BuildFarm);
        game.process_command(Commands::WorkersAuto);

        assert_eq!(game.colony.workers_on_farms, 2);
        assert_eq!(game.colony.free_workers(), 3);
    }

    #[test]
    fn game_w_auto_logs_assignment() {
        let mut game = Game::default();
        game.colony.population = 5;
        game.colony.stone = 100;

        game.process_command(Commands::BuildLumberYard);
        game.process_command(Commands::WorkersAuto);

        assert!(
            game.logs.iter().any(|log| log.contains("Workers:")),
            "expected worker log after w auto, got: {:?}",
            game.logs
        );
    }

    #[test]
    fn game_tick_clamps_workers_after_starvation() {
        let mut game = Game::default();
        game.colony.population = 4;
        game.colony.farms = 2;
        game.colony.food = 0;
        game.colony.workers_on_farms = 4;

        game.tick();

        assert_eq!(game.colony.population, 3);
        assert_eq!(game.colony.workers_on_farms, 3);
        assert_eq!(game.colony.free_workers(), 0);
        assert_eq!(game.colony.staffed_farms(&game.balance), 1);
    }

    #[test]
    fn game_tick_does_not_auto_assign_workers() {
        let mut game = Game::default();
        game.colony.population = 5;
        game.colony.farms = 1;
        game.colony.food = 100;

        game.tick();

        assert_eq!(game.colony.workers_on_farms, 0);
        assert_eq!(game.colony.free_workers(), 5);
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

    #[test]
    fn manual_assign_frees_settlers_without_demolish() {
        let balance = balance();
        let mut colony = colony(4, 2, 0, 0);
        assign(&mut colony, &balance);
        assert_eq!(colony.free_workers(), 0);

        colony.set_workers(WorkerSite::Farm, 0, &balance).unwrap();

        assert_eq!(colony.workers_on_farms, 0);
        assert_eq!(colony.free_workers(), 4);
        assert!(colony.gather_wood(&balance).is_ok());
    }

    #[test]
    fn manual_assign_rejects_too_many_workers() {
        let balance = balance();
        let mut colony = colony(4, 2, 0, 0);
        assert!(colony.set_workers(WorkerSite::Farm, 5, &balance).is_err());
    }

    #[test]
    fn w_auto_fills_workers_once() {
        let balance = balance();
        let mut colony = colony(5, 1, 1, 0);
        colony.set_workers(WorkerSite::Farm, 0, &balance).unwrap();
        colony.set_workers(WorkerSite::Lumber, 0, &balance).unwrap();
        colony.auto_assign_workers(&balance);

        assert_eq!(colony.workers_on_farms, 2);
        assert_eq!(colony.workers_on_lumber_yards, 2);
        assert_eq!(colony.free_workers(), 1);
    }

    #[test]
    fn workers_assignment_persists_through_tick() {
        let mut game = Game::default();
        game.colony.population = 4;
        game.colony.max_population = 4;
        game.colony.farms = 2;
        game.colony
            .set_workers(WorkerSite::Farm, 2, &game.balance)
            .unwrap();
        game.colony.food = 100;

        game.tick();

        assert_eq!(game.colony.workers_on_farms, 2);
        assert_eq!(game.colony.free_workers(), 2);
    }

    #[test]
    fn worker_commands_are_free_actions() {
        assert!(Commands::WorkersAuto.is_worker_management());
        assert!(
            Commands::SetWorkers {
                site: WorkerSite::Farm,
                count: 0
            }
            .is_worker_management()
        );
        assert!(!Commands::GetWood.is_worker_management());
    }

    #[test]
    fn game_w_command_frees_gathering() {
        let mut game = Game::default();
        game.colony.population = 4;
        game.colony.farms = 2;
        assign(&mut game.colony, &game.balance);

        game.process_command(Commands::SetWorkers {
            site: WorkerSite::Farm,
            count: 0,
        });

        assert_eq!(game.colony.free_workers(), 4);
        assert!(
            game.logs.iter().any(|log| log.contains("Set farm workers")),
            "got: {:?}",
            game.logs
        );
    }
}
