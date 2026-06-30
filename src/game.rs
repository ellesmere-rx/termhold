const MAX_LOG_SIZE: usize = 100;

#[derive(PartialEq)]
pub enum Commands {
    GetWood,
    GetStone,
    GetFood,
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
}

impl Colony {
    pub fn gather_wood(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.gather_wood_cost {
            return Err("The workers were unable to gather without additional food.");
        }
        self.food -= balance.gather_wood_cost;
        self.wood += balance.gather_wood_base;
        Ok(balance.gather_wood_base)
    }

    pub fn gather_stone(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.gather_stone_cost {
            return Err("The workers were unable to gather without additional food.");
        }
        self.food -= balance.gather_stone_cost;
        self.stone += balance.gather_stone_base;
        Ok(balance.gather_stone_base)
    }

    pub fn gather_food(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        if self.food < balance.gather_food_cost {
            return Err("Not enough food for food gathering!");
        }
        self.food -= balance.gather_food_cost;
        self.food += balance.gather_food_base;
        Ok(balance.gather_food_base)
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
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            gather_wood_base: 4,
            gather_wood_cost: 1,
            gather_stone_base: 1,
            gather_stone_cost: 1,
            gather_food_base: 8,
            gather_food_cost: 0,
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
        let consumed_food = self.colony.population;
        if self.colony.food < self.colony.population {
            self.colony.food = 0; // съели остатки
            self.logs("Not enough food! Colony is starving, population is decreasing (-1)".into());
            self.colony.population = self.colony.population.saturating_sub(1);
        } else {
            self.colony.food -= self.colony.population;
            self.logs(format!("Colony consumes {} food", consumed_food));
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
