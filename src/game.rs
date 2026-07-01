use rand::RngExt;

const MAX_LOG_SIZE: usize = 100;

#[derive(PartialEq, Debug)]
pub enum Commands {
    GetWood,
    GetStone,
    GetFood,
    BuildHut,
    BuildLumberYard,
    BuildStoneQuarry,
    BuildBarn,
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
    pub max_food: usize,
    pub huts: usize,
    pub barns: usize,
    pub lumber_yards: usize,
    pub stone_quarries: usize,
}

impl Colony {
    pub fn wood_yield(&self, balance: &Balance) -> usize {
        yield_from_pop(balance.gather_wood_base, self.population, 40, 1)
    }

    pub fn stone_yield(&self, balance: &Balance) -> usize {
        yield_from_pop(balance.gather_stone_base, self.population, 33, 1)
    }

    pub fn food_yield(&self, balance: &Balance) -> usize {
        yield_from_pop(balance.gather_food_base, self.population, 50, 1)
    }

    pub fn gather_wood(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        let gather_yield = self.wood_yield(balance);
        self.wood += gather_yield;
        Ok(gather_yield)
    }

    pub fn gather_stone(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        let gather_yield = self.stone_yield(balance);
        self.stone += gather_yield;
        Ok(gather_yield)
    }

    pub fn gather_food(&mut self, balance: &Balance) -> Result<usize, &'static str> {
        let gather_yield = self
            .food_yield(balance)
            .min(self.max_food.saturating_sub(self.food));
        self.food += gather_yield;
        Ok(gather_yield)
    }

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

    pub fn passive_wood(&self, balance: &Balance) -> usize {
        self.lumber_yards * balance.lumber_yard_wood_production
    }

    pub fn passive_stone(&self, balance: &Balance) -> usize {
        self.stone_quarries * balance.stone_quarry_stone_production
    }

    pub fn apply_passive_income(&mut self, balance: &Balance) -> (usize, usize) {
        let wood = self.passive_wood(balance);
        let stone = self.passive_stone(balance);
        self.wood += wood;
        self.stone += stone;
        (wood, stone)
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
            max_food: 25,
            huts: 1,
            barns: 0,
            lumber_yards: 0,
            stone_quarries: 0,
        }
    }
}

pub struct Balance {
    // RESOURCES:
    // Wood
    pub gather_wood_base: usize,
    pub gather_wood_cost: usize,
    // Stone
    pub gather_stone_base: usize,
    pub gather_stone_cost: usize,
    // Food
    pub gather_food_base: usize,
    pub gather_food_cost: usize,
    // Pop
    pub population_increase_cost: usize,
    pub birth_chance_percent: u8,

    // BUILDINGS:
    // Hut
    pub hut_max_population_increase: usize,
    pub build_hut_wood_cost: usize,
    pub build_hut_stone_cost: usize,

    // Lumber
    pub build_lumber_yard_wood_cost: usize,
    pub build_lumber_yard_stone_cost: usize,
    /// +3 wood/day per yard (active g wood ≈ 7–9 at mid pop; yard costs 20w 50s)
    pub lumber_yard_wood_production: usize,
    // Quarry — +2 stone/day per quarry (active g stone ≈ 6–8 at mid pop)
    pub build_stone_quarry_wood_cost: usize,
    pub build_stone_quarry_stone_cost: usize,
    pub stone_quarry_stone_production: usize,

    // Barn base_storage 20 barn_capacity +15 20 wood, 30 stone
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
            gather_wood_cost: 1,
            // Stone
            gather_stone_base: 5,
            gather_stone_cost: 5,
            // Food
            gather_food_base: 5,
            gather_food_cost: 0,
            // Pop
            population_increase_cost: 2,
            birth_chance_percent: 15,

            // BUILDINGS:
            // Hut
            build_hut_wood_cost: 10,
            build_hut_stone_cost: 10,
            hut_max_population_increase: 5,

            // Lumber
            build_lumber_yard_wood_cost: 20,
            build_lumber_yard_stone_cost: 50,

            // Quarry
            build_stone_quarry_wood_cost: 50,
            build_stone_quarry_stone_cost: 20,

            // Barn base_storage 20 barn_capacity +15 20 wood, 30 stone
            barn_max_food_storage_increase: 15,
            build_barn_wood_cost: 20,
            build_barn_stone_cost: 30,
            lumber_yard_wood_production: 3,
            stone_quarry_stone_production: 2,
        }
    }
}

impl Balance {}
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
        } else if self.world.days == 180 {
            println!("Gameover. Colony reached 180 days.");
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

            // Giving birth
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

        // Spoil excess food
        if self.colony.food > self.colony.max_food {
            let spoiled = self.colony.food - self.colony.max_food;
            self.colony.food = self.colony.max_food;
            self.logs(format!(
                "Spoiled {spoiled} food (storage {}/{})",
                self.colony.food, self.colony.max_food
            ));
        }

        // Passive income from buildings (end of day, no food cost)
        let (passive_wood, passive_stone) = self.colony.apply_passive_income(&self.balance);
        if passive_wood > 0 || passive_stone > 0 {
            self.logs(format!(
                "Passive income: +{passive_wood} wood, +{passive_stone} stone"
            ));
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
                    "Huts (+{gain}), max pop +{}, spent {} wood, spent {} stone",
                    self.balance.hut_max_population_increase,
                    self.balance.build_hut_wood_cost,
                    self.balance.build_hut_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildLumberYard => match self.colony.build_lumber_yard(&self.balance) {
                Ok(_) => self.logs(format!(
                    "Lumber yard built (+{} wood/day passive), spent {} wood, spent {} stone",
                    self.balance.lumber_yard_wood_production,
                    self.balance.build_lumber_yard_wood_cost,
                    self.balance.build_lumber_yard_stone_cost
                )),
                Err(msg) => self.logs(msg.to_string()),
            },
            Commands::BuildStoneQuarry => match self.colony.build_stone_quarry(&self.balance) {
                Ok(_) => self.logs(format!(
                    "Stone quarry built (+{} stone/day passive), spent {} wood, spent {} stone",
                    self.balance.stone_quarry_stone_production,
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

fn yield_from_pop(base: usize, pop: usize, percent: usize, min_bonus: usize) -> usize {
    let bonus = (pop * percent / 100).max(min_bonus);
    base + bonus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yeild_from_pop() {
        let base = 5;
        let pop = 2;
        let percent = 40;
        let min_bonus = 1;

        let result = yield_from_pop(base, pop, percent, min_bonus);
        assert_eq!(6, result);
    }

    #[test]
    fn passive_income_scales_with_buildings() {
        let balance = Balance::default();
        let mut colony = Colony::default();
        colony.lumber_yards = 2;
        colony.stone_quarries = 1;

        let (wood, stone) = colony.apply_passive_income(&balance);
        assert_eq!(wood, 6);
        assert_eq!(stone, 2);
    }
}
