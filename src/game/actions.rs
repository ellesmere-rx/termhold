//! Player intents from the UI — domain has no CLI parsing.

use super::BuildingKind;
use super::ResourceKind;

/// Everything the player can do in one turn (before day advance, except free-turn commands).
#[derive(PartialEq, Debug)]
pub enum Actions {
    /// Manual resource gathering using free settlers (`g wood`, …).
    Gather(ResourceKind),
    /// Pay costs and add one building instance (`b farm`, …).
    Build(BuildingKind),
    /// Set total workers on a production type (`w farm 2`, …).
    SetWorkers { kind: BuildingKind, count: usize },
    /// Answer a pending yes/no event (`y` / `n`).
    EventAnswer(bool),
    /// Exit the game loop.
    Quit,
}

impl Actions {
    /// `w` and answering a pending event (`y` / `n`) — no [`super::Game::tick`].
    pub fn is_free_turn(&self) -> bool {
        matches!(self, Actions::SetWorkers { .. } | Actions::EventAnswer(_))
    }
}
