/// Production building type for manual worker assignment (`w` command).
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum WorkerSite {
    Farm,
    Lumber,
    Quarry,
}

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
    SetWorkers { site: WorkerSite, count: usize },
    WorkersAuto,
    Quit,
}

impl Commands {
    /// Worker commands are free — re-render without advancing the day.
    pub fn is_worker_management(&self) -> bool {
        matches!(self, Commands::SetWorkers { .. } | Commands::WorkersAuto)
    }
}
