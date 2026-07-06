//! Colony simulation: state, balance, tick, player actions.

mod actions;
mod balance;
mod building;
mod colony;
mod events;
mod resource;
mod world;

pub use actions::Actions;
pub use balance::Balance;
pub use building::BuildingKind;
pub use colony::Colony;
pub use events::PendingEvent;
pub use resource::ResourceKind;
pub use world::World;

use std::collections::{HashMap, HashSet};

use rand::RngExt;

const MAX_LOG_SIZE: usize = 100;
/// Survive this many days to win.
pub const WIN_DAY: usize = 365;

/// Root game state: colony, calendar, balance tuning, and event log.
pub struct Game {
    /// Resources, population, buildings, worker assignments.
    pub colony: Colony,
    /// Day counter; incremented at end of each [`Self::tick`].
    pub world: World,
    /// Recent log lines (`Day N | …`), capped at [`MAX_LOG_SIZE`].
    pub logs: Vec<String>,
    /// Tunable constants (costs, yields, worker slots, birth rules).
    pub balance: Balance,
    /// Blocked yes/no event — UI accepts only `y`/`n` until answered.
    pub pending_event: Option<PendingEvent>,
    /// [`events::GameEvent::once`] ids that already fired successfully.
    events_fired: HashSet<&'static str>,
    /// Day of last successful answer per event id (for cooldown).
    event_last_day: HashMap<&'static str, usize>,
    /// Set when population hits 0 or day reaches [`WIN_DAY`].
    pub gameover: bool,
}

impl Game {
    /// Start a new run; writes the first log line with `colony_name`.
    pub fn new(colony_name: impl Into<String>) -> Self {
        let name = colony_name.into();
        let mut game = Self {
            colony: Colony::new(name.clone()),
            world: World::default(),
            logs: Vec::with_capacity(100),
            balance: Balance::default(),
            pending_event: None,
            events_fired: HashSet::new(),
            event_last_day: HashMap::new(),
            gameover: false,
        };
        game.logs(format!("{name} makes camp at the frontier."));
        game
    }

    /// Log current aggregate worker counts after `w`.
    fn log_worker_assignment(&mut self) {
        self.logs(format!(
            "Workers: farms {}, lumber {}, quarries {} ({} free for gathering)",
            self.colony.workers_on(BuildingKind::Farm),
            self.colony.workers_on(BuildingKind::LumberYard),
            self.colony.workers_on(BuildingKind::StoneQuarry),
            self.colony.free_workers(),
        ));
    }

    /// After new production building: cap workers to valid slots and population.
    fn clamp_workers_after_build_change(&mut self) {
        self.colony.clamp_workers(&self.balance);
        for msg in self.colony.understaffed_messages(&self.balance) {
            self.logs(msg);
        }
    }

    fn worker_kind_label(kind: BuildingKind) -> &'static str {
        kind.label()
    }

    /// Whether [`Self::process_action`] should clamp workers after handling the action.
    fn should_clamp_workers_after_action(action: &Actions) -> bool {
        matches!(action, Actions::Build(kind) if kind.employs_workers())
    }

    fn log_food_overflow(&mut self, lost: usize) {
        if lost > 0 {
            self.logs(format!("{lost} food did not fit in storage"));
        }
    }

    fn log_build(&mut self, kind: BuildingKind) {
        let b = &self.balance.buildings;
        let msg = match kind {
            BuildingKind::Hut => format!(
                "Hut built (+1), max pop +{}, spent {} wood, {} stone",
                b.hut_max_population_increase, b.build_hut_wood_cost, b.build_hut_stone_cost
            ),
            BuildingKind::Barn => format!(
                "Barn built (+{} food storage, max {}), spent {} wood, {} stone",
                b.barn_max_food_storage_increase,
                self.colony.max_food,
                b.build_barn_wood_cost,
                b.build_barn_stone_cost
            ),
            BuildingKind::Farm => format!(
                "Farm built (+{} food/day when {} workers assigned), spent {} wood, {} stone",
                b.farm_food_production,
                b.farm_max_workers,
                b.build_farm_wood_cost,
                b.build_farm_stone_cost
            ),
            BuildingKind::LumberYard => format!(
                "Lumber yard built (+{} wood/day when {} workers assigned), spent {} wood, {} stone",
                b.lumber_yard_wood_production,
                b.lumber_yard_max_workers,
                b.build_lumber_yard_wood_cost,
                b.build_lumber_yard_stone_cost
            ),
            BuildingKind::StoneQuarry => format!(
                "Stone quarry built (+{} stone/day when {} workers assigned), spent {} wood, {} stone",
                b.stone_quarry_stone_production,
                b.stone_quarry_max_workers,
                b.build_stone_quarry_wood_cost,
                b.build_stone_quarry_stone_cost
            ),
        };
        self.logs(msg);
    }

    fn check_end_conditions(&mut self) {
        if self.colony.population == 0 {
            if !self.gameover {
                println!("Gameover. Colony is dead.");
            }
            self.gameover = true;
        } else if self.world.days >= WIN_DAY {
            println!("Victory! Colony survived {WIN_DAY} days.");
            self.gameover = true;
        }
    }

    /// True if this event cannot roll yet (`once` already fired, or cooldown active).
    fn event_blocked(&self, event: &events::GameEvent) -> bool {
        if event.once && self.events_fired.contains(event.id) {
            return true;
        }
        if let Some(cooldown) = event.cooldown_days
            && let Some(&last) = self.event_last_day.get(event.id)
                && self.world.days.saturating_sub(last) < cooldown {
                    return true;
                }
        false
    }

    /// Record that this event completed (`once` / cooldown tracking).
    fn mark_event_answered(&mut self, event: &events::GameEvent) {
        if event.once {
            self.events_fired.insert(event.id);
        }
        if event.once || event.cooldown_days.is_some() {
            self.event_last_day.insert(event.id, self.world.days);
        }
    }

    /// `exact_day` match, else `chance_percent` roll.
    fn event_triggered(&self, event: &events::GameEvent, roll: u8) -> bool {
        if event.exact_day == Some(self.world.days) {
            return true;
        }
        event.chance_percent > 0 && roll < event.chance_percent
    }

    /// Apply a rolled event: Choice sets pending, Auto runs immediately.
    fn fire_event(&mut self, event: &'static events::GameEvent) {
        self.logs(format!("Event: {}", event.title));
        match event.effect {
            events::EventEffect::Choice { .. } => {
                self.pending_event = Some(PendingEvent { event_id: event.id });
            }
            events::EventEffect::Auto(apply) => match apply(&mut self.colony, &self.balance) {
                Ok(msg) => {
                    self.logs(msg.to_string());
                    self.mark_event_answered(event);
                    self.check_end_conditions();
                }
                Err(msg) => self.logs(msg.to_string()),
            },
        }
    }

    /// Roll random events at end of day. See [`events`] module docs for selection rules.
    fn try_roll_event(&mut self) {
        if self.pending_event.is_some() || self.gameover {
            return;
        }

        let mut rng = rand::rng();
        for event in events::EVENTS {
            if self.world.days < event.min_day {
                continue;
            }
            if let Some(kind) = event.requires_building
                && self.colony.count(kind) == 0 {
                    continue;
                }
            if self.event_blocked(event) {
                continue;
            }
            let roll: u8 = rng.random_range(0..100);
            if self.event_triggered(event, roll) {
                self.fire_event(event);
                return;
            }
        }
    }

    /// Run `on_yes` / `on_no` for the pending event and write the returned log line.
    ///
    /// On `Err` the event is restored — player must answer again. Most events use `Ok`
    /// even when the colony cannot pay (they apply the **no** outcome inline).
    fn resolve_event_answer(&mut self, yes: bool) {
        let Some(pending) = self.pending_event.take() else {
            return;
        };
        let Some(event) = events::find_event(pending.event_id) else {
            return;
        };

        let events::EventEffect::Choice { on_yes, on_no, .. } = event.effect else {
            return;
        };

        let apply = if yes { on_yes } else { on_no };
        match apply(&mut self.colony, &self.balance) {
            Ok(log_text) => {
                self.logs(log_text.to_string());
                self.mark_event_answered(event);
                self.check_end_conditions();
            }
            Err(error_text) => {
                self.logs(error_text.to_string());
                self.pending_event = Some(pending);
            }
        }
    }

    /// Full event definition for UI while a Choice event is pending.
    pub fn pending_event_def(&self) -> Option<&'static events::GameEvent> {
        self.pending_event
            .as_ref()
            .and_then(|pending| events::find_event(pending.event_id))
    }

    /// End-of-day simulation: food, births, spoilage, passive income, day++, then event roll.
    pub fn tick(&mut self) {
        if self.colony.population == 0 {
            self.check_end_conditions();
            return;
        }

        if self.world.days >= WIN_DAY {
            println!("Victory! Colony survived {WIN_DAY} days.");
            self.gameover = true;
            return;
        }

        let pop = self.colony.population;
        let rations = self.colony.food.min(pop);
        self.colony.food -= rations;
        let deficit = pop - rations;

        if deficit == 0 {
            self.colony.starvation_days = 0;
            self.logs(format!("Colony consumes {rations} food"));

            if self.colony.population >= self.balance.population.min_population_for_birth
                && self.colony.population < self.colony.max_population
            {
                let min_food = self.colony.population + self.balance.population.increase_cost;
                if self.colony.food >= min_food {
                    let chance = self.balance.population.birth_chance_percent;
                    let mut rng = rand::rng();
                    let roll: u8 = rng.random_range(0..100);
                    if roll < chance {
                        self.colony.food = self
                            .colony
                            .food
                            .saturating_sub(self.balance.population.increase_cost);
                        self.colony.population += 1;
                        self.logs(format!(
                            "Birth! population +1 (chance {chance}%), food -{}",
                            self.balance.population.increase_cost
                        ));
                    }
                }
            }
        } else {
            self.colony.starvation_days += 1;
            self.logs(format!(
                "Hungry: fed {rations}/{pop} ({deficit} unfed, day {} of {})",
                self.colony.starvation_days, self.balance.population.starvation_days_to_death,
            ));

            let mut rng = rand::rng();
            let roll: u8 = rng.random_range(0..100);
            let chance = self
                .balance
                .population
                .starvation_death_chance_percent
                .saturating_mul(deficit as u8)
                .min(100);

            let guaranteed =
                self.colony.starvation_days >= self.balance.population.starvation_days_to_death;
            if guaranteed || roll < chance {
                self.colony.population = self.colony.population.saturating_sub(1);
                self.logs(if guaranteed {
                    "A settler died of starvation (prolonged hunger).".into()
                } else {
                    format!("A settler died of starvation ({chance}% roll failed).")
                });
                if self.colony.population == 0 {
                    self.check_end_conditions();
                    self.world.days += 1;
                    return;
                }
            }
        }

        if self.colony.food > self.colony.max_food {
            let spoiled = self.colony.food - self.colony.max_food;
            self.colony.food = self.colony.max_food;
            self.logs(format!(
                "Spoiled {spoiled} food (storage {}/{})",
                self.colony.food, self.colony.max_food
            ));
        }

        self.colony.clamp_workers(&self.balance);

        let (_, _, _, food_lost) = self.colony.apply_passive_income(&self.balance);
        self.log_food_overflow(food_lost);

        self.world.days += 1;
        self.check_end_conditions();
        self.try_roll_event();
    }

    /// Apply one player action. [`Actions::is_free_turn`] commands skip the day in the UI loop.
    pub fn process_action(&mut self, action: Actions) {
        let clamp_after = Self::should_clamp_workers_after_action(&action);

        match action {
            Actions::Gather(kind) => {
                let food_lost = if kind == ResourceKind::Food {
                    self.colony
                        .food_yield(&self.balance)
                        .saturating_sub(self.colony.max_food.saturating_sub(self.colony.food))
                } else {
                    0
                };
                match self.colony.gather(kind, &self.balance) {
                    Ok(gain) => {
                        self.logs(format!(
                            "Gathered {} (+{gain}) with {} free settler(s)",
                            kind.label(),
                            self.colony.free_workers()
                        ));
                        self.log_food_overflow(food_lost);
                    }
                    Err(msg) => self.logs(msg.to_string()),
                }
            }
            Actions::Build(kind) => match self.colony.build(kind, &self.balance) {
                Ok(_) => self.log_build(kind),
                Err(msg) => self.logs(msg.to_string()),
            },
            Actions::SetWorkers { kind, count } => {
                match self.colony.set_workers(kind, count, &self.balance) {
                    Ok(()) => {
                        self.logs(format!(
                            "Set {} workers to {}.",
                            Self::worker_kind_label(kind),
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

            Actions::EventAnswer(yes) => self.resolve_event_answer(yes),
            Actions::Quit => {}
        }

        if clamp_after {
            self.clamp_workers_after_build_change();
        }
    }

    /// Append a log line prefixed with current day; drop oldest if over [`MAX_LOG_SIZE`].
    pub fn logs(&mut self, text: String) {
        self.logs
            .push(format!("Day {} | {}", self.world.days, text));

        let extra = self.logs.len().saturating_sub(MAX_LOG_SIZE);
        self.logs.drain(..extra);
    }
}
