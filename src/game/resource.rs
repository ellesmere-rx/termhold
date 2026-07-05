use super::commands::Commands;

/// Basic colony resources gathered with `g wood` / `g stone` / `g food`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Wood,
    Stone,
    Food,
}

impl ResourceKind {
    pub const ALL: [Self; 3] = [Self::Wood, Self::Stone, Self::Food];

    pub fn label(self) -> &'static str {
        match self {
            Self::Wood => "wood",
            Self::Stone => "stone",
            Self::Food => "food",
        }
    }

    /// Free-settler bonus percent used in gather yield formula.
    pub fn gather_percent(self) -> usize {
        match self {
            Self::Wood => 40,
            Self::Stone => 33,
            Self::Food => 50,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "wood" => Some(Self::Wood),
            "stone" => Some(Self::Stone),
            "food" => Some(Self::Food),
            _ => None,
        }
    }

    pub fn from_gather_command(command: &Commands) -> Option<Self> {
        match command {
            Commands::GetWood => Some(Self::Wood),
            Commands::GetStone => Some(Self::Stone),
            Commands::GetFood => Some(Self::Food),
            _ => None,
        }
    }

    pub fn to_gather_command(self) -> Commands {
        match self {
            Self::Wood => Commands::GetWood,
            Self::Stone => Commands::GetStone,
            Self::Food => Commands::GetFood,
        }
    }
}
