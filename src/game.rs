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

impl World {
    // pub fn new() -> Self {
    //     // Self {
    //         days: 0,
    //         wood: todo!(),
    //     }
    // }
}

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

impl Colony {}

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

pub struct Game {
    pub colony: Colony,
    pub world: World,
    pub logs: Vec<String>,
    pub gameover: bool,
}

impl Game {
    // pub fn new() -> Self {
    // Self {
    //     colony: Colony::new("New Haven".to_string()),
    //     world: World::new(),
    // }
    // todo!()
    // }

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
            Commands::GetWood => {
                self.logs("Gathered wood (+2)".to_string());
                self.colony.wood += 4;
            }
            Commands::GetStone => {
                self.logs("Gathered stone (+1)".to_string());
                self.colony.stone += 1;
            }
            Commands::GetFood => {
                self.logs("Gathered food (+2)".to_string());
                self.colony.food += 8;
            }
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
            gameover: false,
        }
    }
}
