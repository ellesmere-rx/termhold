/// Global time — only `days` for now. Incremented at the very end of each [`Game::tick`].
pub struct World {
    pub days: usize,
}
impl World {}

impl Default for World {
    fn default() -> Self {
        Self { days: 1 }
    }
}
