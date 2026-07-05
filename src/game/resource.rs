//! Gatherable and storable resources.

/// Resources the colony can gather actively or produce passively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Wood,
    Stone,
    Food,
}

impl ResourceKind {
    /// Short label for logs and UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Wood => "wood",
            Self::Stone => "stone",
            Self::Food => "food",
        }
    }

    /// Bonus per free settler in [`Colony::yield_from_pop`](super::Colony::yield_from_pop).
    pub fn gather_percent(self) -> usize {
        match self {
            Self::Wood => 40,
            Self::Stone => 33,
            Self::Food => 50,
        }
    }
}
