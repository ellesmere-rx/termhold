const MAX_LOG_SIZE: usize = 100;

#[derive(PartialEq)]
pub enum Commands {
    GetWood,
    GetStone,
    GetFood,
    BuildHut,
    BuildLumberYard,
    // BuildStoneQuarry,
    // BuildGranary,
    Quit,
}

pub struct World {
    pub days: usize,
}

impl World {}

impl Default for World {
    fn default() -> Self {
        Self { days: 1 }
    }
}

pub struct Colony {
    pub name: String,
    pub wood: usize,
    pub stone: usize,
    pub food: usize,
    pub population: usize,
    pub max_population: usize,
    pub huts: usize,
    //    pub granaries: usize,
    pub lumber_yards: usize,
    //    pub stone_quarries: usize,
}

impl Colony {
    pub fn gather_wood(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.gather_wood_cost {
            return Err("The workers were unable to gather without additional food.");
        }
        self.food -= balance.gather_wood_cost;
        self.wood += balance.gather_wood_base * self.population * self.lumber_yards;
        Ok(balance.gather_wood_base * self.population * self.lumber_yards)
    }

    pub fn gather_stone(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.gather_stone_cost {
            return Err("The workers were unable to gather without additional food.");
        }
        self.food -= balance.gather_stone_cost;
        self.stone += balance.gather_stone_base * self.population;
        Ok(balance.gather_stone_base * self.population)
    }

    pub fn gather_food(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.gather_food_cost {
            return Err("Not enough food for food gathering!");
        }
        self.food -= balance.gather_food_cost;
        self.food += balance.gather_food_base * self.population;
        Ok(balance.gather_food_base * self.population)
    }

    pub fn build_hut(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.build_hut_food_cost {
            return Err("Not enough food to build a hut!");
        }
        if self.wood < balance.build_hut_wood_cost {
            return Err("Not enough wood to build a hut!");
        }
        self.food -= balance.build_hut_food_cost;
        self.wood -= balance.build_hut_wood_cost;
        self.huts += 1;
        self.max_population += balance.hut_max_population_increase;
        Ok(1)
    }
    pub fn build_lumber_yard(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.build_lumber_yard_food_cost {
            return Err("Not enough food to build a lumber yard!");
        }
        if self.wood < balance.build_lumber_yard_wood_cost {
            return Err("Not enough wood to build a hut!");
        }
        if self.stone < balance.build_lumber_yard_stone_cost {
            return Err("Not enough stone to build a lumber yard!");
        }
        self.food -= balance.build_lumber_yard_food_cost;
        self.wood -= balance.build_lumber_yard_wood_cost;
        self.stone -= balance.build_lumber_yard_stone_cost;
        self.lumber_yards += 1;
        Ok(1)
    }
}

impl Default for Colony {
    fn default() -> Self {
        Self {
            name: "Default colony".to_string(),
            wood: 50,
            stone: 30,
            food: 25,
            population: 5,
            max_population: 5,
            huts: 1,
            //            granaries: 0,
            lumber_yards: 1,
            //            stone_quarries: 0,
        }
    }
}

pub struct Balance {
    pub gather_wood_base: usize,
    pub gather_wood_cost: usize,
    pub gather_stone_base: usize,
    pub gather_stone_cost: usize,
    pub gather_food_base: usize,
    pub gather_food_cost: usize,
    pub population_increase_cost: usize,

    pub build_hut_wood_cost: usize,
    pub build_hut_food_cost: usize,
    pub hut_max_population_increase: usize,

    pub build_lumber_yard_wood_cost: usize,
    pub build_lumber_yard_food_cost: usize,
    pub build_lumber_yard_stone_cost: usize,
    //    pub build_stone_quarry_wood_cost: usize,
    //    pub build_stone_quarry_food_cost: usize,
    //    pub build_stone_quarry_stone_cost: usize,

    //    pub build_granary_wood_cost: usize,
    //    pub build_granary_food_cost: usize,
    //    pub build_granary_stone_cost: usize,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            gather_wood_base: 5,
            gather_wood_cost: 1,
            gather_stone_base: 5,
            gather_stone_cost: 5,
            gather_food_base: 5,
            gather_food_cost: 0,
            build_hut_wood_cost: 10,
            hut_max_population_increase: 5,
            population_increase_cost: 5,
            build_hut_food_cost: 5,
            build_lumber_yard_wood_cost: 20,
            build_lumber_yard_food_cost: 10,
            build_lumber_yard_stone_cost: 50,
            //            build_stone_quarry_wood_cost: 50,
            //            build_stone_quarry_food_cost: 10,
            //            build_stone_quarry_stone_cost: 20,
            //            build_granary_wood_cost: 100,
            //            build_granary_food_cost: 20,
            //            build_granary_stone_cost: 100,
        }
    }
}

pub struct Game {
    pub colony: Colony,
    pub world: World,
    pub logs: Vec<String>,
    pub balance: Balance,
    pub gameover: bool,
}

impl Game {
    pub fn tick(&mut self) {
        // Gameover check
        if self.colony.population == 0 {
            println!("Gameover. Colony is dead.");
            self.gameover = true;
        }

        // Colony consumes food
        if self.colony.food < self.colony.population {
            self.colony.food = 0; // съели остатки
            self.logs("Not enough food! Colony is starving, population is decreasing (-1)".into());
            self.colony.population = self.colony.population.saturating_sub(1);
        } else {
            self.colony.food -= self.colony.population;
            self.logs(format!("Colony consumes {} food", self.colony.population));
            if self.colony.food >= self.balance.population_increase_cost
                && self.colony.population + 1 <= self.colony.max_population
            {
                self.colony.food = self
                    .colony
                    .food
                    .saturating_sub(self.balance.population_increase_cost);
                self.colony.population += 1;
                self.logs(format!(
                    "population +{} food -{}",
                    1, self.balance.population_increase_cost
                ));
            }
        }

        // Days
        self.world.days += 1;
    }

    pub fn process_command(&mut self, command: Commands) {
        match command {
            Commands::GetWood => match self.colony.gather_wood(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Gathered wood (+{gain}), spent {} food",
                    self.balance.gather_wood_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::GetStone => match self.colony.gather_stone(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Gathered stone (+{gain}), spent {} food",
                    self.balance.gather_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::GetFood => match self.colony.gather_food(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Gathered food (+{gain}), spent {} food",
                    self.balance.gather_food_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildHut => match self.colony.build_hut(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Huts (+{gain}), max pop +{}, spent {} food, spent {} wood",
                    self.balance.hut_max_population_increase,
                    self.balance.build_hut_food_cost,
                    self.balance.build_hut_wood_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildLumberYard => match self.colony.build_lumber_yard(&self.balance) {
                Ok(gain) => self.logs(format!(
                    "Lumber yards (+{gain}), wood gathering multiplier (+{gain}), spent {} food, spent {} wood, spent {} stone",
                    self.balance.build_lumber_yard_food_cost,
                    self.balance.build_lumber_yard_wood_cost,
                    self.balance.build_lumber_yard_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::Quit => {}
        }
    }

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
