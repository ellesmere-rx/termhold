use crate::game::ResourceKind;

pub struct GatherBalance {
    // --- Active gathering ---
    /// Base resources before free-settler bonus.
    pub gather_wood_base: usize,
    pub gather_stone_base: usize,
    pub gather_food_base: usize,
}

impl GatherBalance {
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
            gather_wood_base: 5,
            gather_stone_base: 5,
            gather_food_base: 5,
        }
    }
}
