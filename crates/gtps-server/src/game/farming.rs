use gtps_world::{Tile, World};
use gtps_world::tile::{Tree, FLAG_HAS_EXTRA};

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_secs())
}

pub fn plant(seed: u16, background: u16, grow_seconds: u32, now: u64) -> Tile {
    let mut tile = Tile::new(seed, background);
    tile.flags |= FLAG_HAS_EXTRA;
    tile.tree = Some(Tree { planted_at: now, grow_seconds: Some(grow_seconds), fruit: 1 });
    tile
}

pub fn grow_all(world: &mut World, now: u64) -> Vec<usize> {
    let mut changed = Vec::new();
    for (index, tile) in world.tiles.iter_mut().enumerate() {
        if let Some(tree) = tile.tree.as_mut() {
            let Some(duration) = tree.grow_seconds else { continue; };
            if !tree.mature(now) {
                tree.planted_at = now.saturating_sub(u64::from(duration));
                changed.push(index);
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spray_matures_trees_without_turning_them_into_blocks() {
        let mut world = World::sized("SPRAY", 200, 200);
        world.set_tile(199, 199, plant(10341, 14, 240, 1000));
        world.set_tile(0, 0, Tile::new(2, 0));
        assert_eq!(grow_all(&mut world, 1001), vec![39999]);
        let tile = world.tile(199, 199).unwrap();
        assert_eq!(tile.foreground, 10341);
        assert_eq!(tile.background, 14);
        assert!(tile.tree.unwrap().mature(1001));
        assert!(grow_all(&mut world, 1001).is_empty());
        assert_eq!(world.tile(0, 0).unwrap().foreground, 2);
    }
}
