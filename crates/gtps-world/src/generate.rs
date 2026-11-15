use crate::items;
use crate::tile::Tile;
use crate::world::{World, DOOR_ROW, HEIGHT, WIDTH};

#[derive(Debug, Clone)]
pub struct Generator {
    state: u32,
}

impl Generator {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut state = self.state;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.state = state;
        state
    }

    pub fn range(&mut self, max: u32) -> u32 {
        if max == 0 {
            0
        } else {
            self.next_u32() % max
        }
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        self.range(100) < percent
    }
}

#[must_use]
pub fn seed_from_name(name: &str) -> u32 {
    gtps_proto::hash_data(name.as_bytes())
}

#[must_use]
pub fn default_world(name: &str) -> World {
    let mut world = World::empty(name);
    let mut rng = Generator::new(seed_from_name(name));

    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let (foreground, background) = layer_for(&mut rng, y);
            let mut tile = Tile::new(foreground, background);
            tile.flags = background_flags(background);
            world.set_tile(x, y, tile);
        }
    }

    let door_x = WIDTH / 2;
    world.set_tile(door_x, DOOR_ROW, Tile::new(items::MAIN_DOOR, 0));

    world
}

fn layer_for(rng: &mut Generator, y: usize) -> (u16, u16) {
    match y {
        0..=23 => (0, cave_or_air(rng)),
        DOOR_ROW => (items::BEDROCK, 0),
        25..=35 => (items::DIRT, cave_or_air(rng)),
        36..=53 => (items::ROCK, cave_or_air(rng)),
        _ => (items::BEDROCK, 0),
    }
}

fn cave_or_air(rng: &mut Generator) -> u16 {
    if rng.chance(45) {
        items::CAVE_BACKGROUND
    } else {
        0
    }
}

fn background_flags(background: u16) -> u16 {
    if background == 0 {
        0
    } else {
        crate::tile::FLAG_PUBLIC
    }
}
