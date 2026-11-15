use std::collections::HashMap;

use gtps_world::{default_world, World};

use crate::game::locks::Magplant;

#[derive(Debug)]
pub struct WorldEntry {
    pub world: World,
    pub magplants: Vec<Magplant>,
}

#[derive(Debug, Default)]
pub struct WorldStore {
    worlds: HashMap<String, WorldEntry>,
}

impl WorldStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&World> {
        self.worlds.get(name).map(|entry| &entry.world)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut World> {
        self.worlds.get_mut(name).map(|entry| &mut entry.world)
    }

    pub fn ensure(&mut self, name: &str) -> &mut World {
        &mut self.entry_mut(name).world
    }

    #[must_use]
    pub fn is_loaded(&self, name: &str) -> bool {
        self.worlds.contains_key(name)
    }

    pub fn entry_mut(&mut self, name: &str) -> &mut WorldEntry {
        self.worlds
            .entry(name.to_string())
            .or_insert_with(|| WorldEntry {
                world: default_world(name),
                magplants: Vec::new(),
            })
    }

    pub fn ensure_magplant(&mut self, name: &str, tile: usize, lock_item: u16) -> &mut Magplant {
        let entry = self.entry_mut(name);
        let index = if let Some(index) = entry.magplants.iter().position(|plant| plant.tile == tile)
        {
            entry.magplants[index] = Magplant::new(tile, lock_item);
            index
        } else {
            entry.magplants.push(Magplant::new(tile, lock_item));
            entry.magplants.len() - 1
        };

        &mut entry.magplants[index]
    }

    pub fn remove_magplant(&mut self, name: &str, tile: usize) {
        if let Some(entry) = self.worlds.get_mut(name) {
            entry.magplants.retain(|plant| plant.tile != tile);
        }
    }

    #[must_use]
    pub fn take_from_magplant(
        &mut self,
        name: &str,
        preferred: Option<usize>,
        allow_grab: bool,
        amount: u32,
        reach: Option<(usize, usize, usize)>,
    ) -> Option<(usize, u16, u32)> {
        let entry = self.worlds.get_mut(name)?;
        let count = entry.magplants.len();
        if count == 0 {
            return None;
        }

        let start = preferred.and_then(|tile| entry.magplants.iter().position(|p| p.tile == tile));

        let candidates = (0..count).map(|step| match start {
            Some(index) if allow_grab => (index + step) % count,
            Some(index) => index,
            None => step,
        });

        for index in candidates {
            if !within_reach(entry.magplants[index].tile, reach) {
                continue;
            }

            let plant = entry.magplants.get_mut(index)?;
            if !plant.is_empty() {
                let taken = plant.take(amount);
                return Some((plant.tile, plant.lock_item, taken));
            }
        }

        None
    }
}

fn within_reach(tile: usize, reach: Option<(usize, usize, usize)>) -> bool {
    let Some((origin_x, origin_y, range)) = reach else {
        return true;
    };

    let Some((x, y)) = World::coords(tile) else {
        return false;
    };

    origin_x.abs_diff(x) <= range && origin_y.abs_diff(y) <= range
}

#[cfg(test)]
mod tests {
    use super::WorldStore;
    use crate::game::locks::{by_kind, LockKind};

    #[test]
    fn magplant_stock_tracks_lock() {
        let mut store = WorldStore::new();
        let diamond = by_kind(LockKind::Diamond).item_id;

        store.ensure_magplant("TEST", 10, diamond);
        let (tile, lock, taken) = store
            .take_from_magplant("TEST", None, false, 3, None)
            .expect("stock");

        assert_eq!(tile, 10);
        assert_eq!(lock, diamond);
        assert_eq!(taken, 3);
    }

    #[test]
    fn auto_grab_moves_to_the_next_stocked_plant() {
        let mut store = WorldStore::new();
        let world = by_kind(LockKind::World).item_id;

        store.ensure_magplant("TEST", 1, world);
        store.ensure_magplant("TEST", 2, world);

        let first = store
            .take_from_magplant("TEST", None, true, 5_000, None)
            .expect("first");
        assert_eq!(first.0, 1);

        let second = store
            .take_from_magplant("TEST", Some(1), true, 5_000, None)
            .expect("second");
        assert_eq!(second.0, 2);

        let none = store.take_from_magplant("TEST", Some(2), true, 5_000, None);
        assert!(none.is_none());
    }

    #[test]
    fn fist_range_limits_reachable_magplants() {
        let mut store = WorldStore::new();
        let world = by_kind(LockKind::World).item_id;

        store.ensure_magplant("TEST", 100, world);
        store.ensure_magplant("TEST", 102, world);

        let stuck = store.take_from_magplant("TEST", Some(102), false, 1, Some((0, 1, 1)));
        assert!(stuck.is_none());

        let grab = store.take_from_magplant("TEST", Some(102), true, 1, Some((0, 1, 1)));
        assert_eq!(grab.map(|out| out.0), Some(100));

        let wide = store.take_from_magplant("TEST", Some(102), false, 1, Some((0, 1, 3)));
        assert_eq!(wide.map(|out| out.0), Some(102));
    }

    #[test]
    fn removing_a_lock_drops_its_magplant() {
        let mut store = WorldStore::new();
        store.ensure_magplant("TEST", 7, by_kind(LockKind::Royal).item_id);
        assert!(store
            .take_from_magplant("TEST", Some(7), false, 1, None)
            .is_some());

        store.remove_magplant("TEST", 7);
        assert!(store
            .take_from_magplant("TEST", Some(7), false, 1, None)
            .is_none());
    }
}
