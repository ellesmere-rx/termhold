//! Player intents from the UI — domain has no CLI parsing.

use super::BuildingKind;
use super::ResourceKind;

/// Everything the player can do in one turn (before day advance, except worker commands).
#[derive(PartialEq, Debug)]
pub enum Actions {
    /// Manual resource gathering using free settlers (`g wood`, …).
    Gather(ResourceKind),
    /// Pay costs and add one building instance (`b farm`, …).
    Build(BuildingKind),
    /// Set total workers on a production type (`w farm 2`, …).
    SetWorkers { kind: BuildingKind, count: usize },
    /// Exit the game loop.
    Quit,
}

impl Actions {
    /// Worker commands do not advance the day (handled in UI loop).
    pub fn is_worker_management(&self) -> bool {
        matches!(self, Actions::SetWorkers { .. })
    }
}
