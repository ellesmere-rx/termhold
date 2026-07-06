//! Active gathering balance (base yields before free-settler bonus).

use crate::game::ResourceKind;

/// Base gather amounts before [`ResourceKind::gather_percent`] scaling.
pub struct GatherBalance {
    /// Wood from `g wood` with one free settler (before percent bonus).
    pub gather_wood_base: usize,
    /// Stone from `g stone`.
    pub gather_stone_base: usize,
    /// Food from `g food`.
    pub gather_food_base: usize,
}

impl GatherBalance {
    /// Base yield for `kind` (see field docs above).
    pub fn base(&self, kind: ResourceKind) -> usize {
        match kind {
            ResourceKind::Wood => self.gather_wood_base,
            ResourceKind::Stone => self.gather_stone_base,
            ResourceKind::Food => self.gather_food_base,
        }
    }
}

impl Default for GatherBalance {
    fn default() -> Self {
        Self {
            gather_wood_base: 3,
            gather_stone_base: 3,
            gather_food_base: 5,
        }
    }
}
