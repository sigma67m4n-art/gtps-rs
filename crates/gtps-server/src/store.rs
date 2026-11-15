use std::collections::HashMap;

use gtps_world::{default_world, World};

#[derive(Debug, Default)]
pub struct WorldStore {
    worlds: HashMap<String, World>,
}

impl WorldStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&World> {
        self.worlds.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut World> {
        self.worlds.get_mut(name)
    }

    pub fn ensure(&mut self, name: &str) -> &mut World {
        self.worlds
            .entry(name.to_string())
            .or_insert_with(|| default_world(name))
    }

    #[must_use]
    pub fn is_loaded(&self, name: &str) -> bool {
        self.worlds.contains_key(name)
    }
}
