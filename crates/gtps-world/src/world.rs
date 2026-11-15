use crate::tile::Tile;

pub const WIDTH: usize = 100;
pub const HEIGHT: usize = 60;
pub const TILE_COUNT: usize = WIDTH * HEIGHT;
pub const FORMAT_VERSION: u16 = 0x14;
pub const TILE_SIZE_PIXELS: f32 = 32.0;
pub const PIXEL_WIDTH: f32 = WIDTH as f32 * TILE_SIZE_PIXELS;
pub const PIXEL_HEIGHT: f32 = HEIGHT as f32 * TILE_SIZE_PIXELS;
pub const DOOR_ROW: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundObject {
    pub item_id: u16,
    pub pos_x: f32,
    pub pos_y: f32,
    pub count: u16,
    pub uid: u32,
}

#[derive(Debug, Clone)]
pub struct World {
    pub name: String,
    pub tiles: Vec<Tile>,
    pub objects: Vec<GroundObject>,
    pub next_object_uid: u32,
    pub base_weather: u16,
    pub current_weather: u16,
    pub owner_user_id: u32,
    pub flags: u32,
    pub main_lock: i32,
}

impl World {
    #[must_use]
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tiles: vec![Tile::default(); TILE_COUNT],
            objects: Vec::new(),
            next_object_uid: 1,
            base_weather: 0,
            current_weather: 0,
            owner_user_id: 0,
            flags: 0,
            main_lock: -1,
        }
    }

    #[must_use]
    pub fn index(x: usize, y: usize) -> Option<usize> {
        (x < WIDTH && y < HEIGHT).then_some(y * WIDTH + x)
    }

    #[must_use]
    pub fn coords(index: usize) -> Option<(usize, usize)> {
        (index < TILE_COUNT).then_some((index % WIDTH, index / WIDTH))
    }

    #[must_use]
    pub fn tile(&self, x: usize, y: usize) -> Option<&Tile> {
        Self::index(x, y).and_then(|index| self.tiles.get(index))
    }

    #[must_use]
    pub fn tile_at(&self, index: usize) -> Option<&Tile> {
        self.tiles.get(index)
    }

    pub fn tile_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
        Self::index(x, y).and_then(|index| self.tiles.get_mut(index))
    }

    pub fn set_tile(&mut self, x: usize, y: usize, tile: Tile) {
        if let Some(index) = Self::index(x, y) {
            self.tiles[index] = tile;
        }
    }

    pub fn add_object(&mut self, item_id: u16, pos_x: f32, pos_y: f32, count: u16) -> u32 {
        let uid = self.next_object_uid;
        self.next_object_uid = self.next_object_uid.wrapping_add(1);
        self.objects.push(GroundObject {
            item_id,
            pos_x,
            pos_y,
            count,
            uid,
        });
        uid
    }

    pub fn remove_object(&mut self, uid: u32) -> Option<GroundObject> {
        let index = self.objects.iter().position(|object| object.uid == uid)?;
        Some(self.objects.remove(index))
    }

    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
}
