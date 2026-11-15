use crate::cursor::{Reader, Result, Writer};

pub const FLAG_EXTENDED: u32 = 0x08;

pub const TYPE_STATE: u8 = 0;
pub const TYPE_CALL_FUNCTION: u8 = 1;
pub const TYPE_UPDATE_STATUS: u8 = 2;
pub const TYPE_TILE_CHANGE_REQUEST: u8 = 3;
pub const TYPE_SEND_MAP_DATA: u8 = 4;
pub const TYPE_SEND_TILE_UPDATE_DATA: u8 = 5;
pub const TYPE_SEND_TILE_UPDATE_DATA_MULTIPLE: u8 = 6;
pub const TYPE_TILE_ACTIVATE_REQUEST: u8 = 7;
pub const TYPE_TILE_APPLY_DAMAGE: u8 = 8;
pub const TYPE_SEND_INVENTORY_STATE: u8 = 9;
pub const TYPE_ITEM_ACTIVATE_REQUEST: u8 = 10;
pub const TYPE_ITEM_ACTIVATE_OBJECT_REQUEST: u8 = 11;
pub const TYPE_SEND_TILE_TREE_STATE: u8 = 12;
pub const TYPE_MODIFY_ITEM_INVENTORY: u8 = 13;
pub const TYPE_ITEM_CHANGE_OBJECT: u8 = 14;
pub const TYPE_SEND_LOCK: u8 = 15;
pub const TYPE_SEND_ITEM_DATABASE_DATA: u8 = 16;
pub const TYPE_SEND_PARTICLE_EFFECT: u8 = 17;
pub const TYPE_SET_ITEM_ASYNC: u8 = 18;
pub const TYPE_SET_ITEM_ASYNC_MULTIPLE: u8 = 19;
pub const TYPE_SEND_PLAYER_TRIBUTE_DATA: u8 = 20;
pub const TYPE_PING_REPLY: u8 = 21;
pub const TYPE_PING_REQUEST: u8 = 22;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TankPacket {
    pub kind: u8,
    pub pad1: u8,
    pub pad2: u8,
    pub pad3: u8,
    pub net_id: i32,
    pub secondary_id: i32,
    pub flags: u32,
    pub float1: f32,
    pub int_data: i32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub float2: f32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub data_size: u32,
}

impl TankPacket {
    pub const SIZE: usize = 56;

    #[must_use]
    pub fn new(kind: u8) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    pub fn decode(reader: &mut Reader<'_>) -> Result<Self> {
        let mut packet = Self::default();
        packet.kind = reader.u8()?;
        packet.pad1 = reader.u8()?;
        packet.pad2 = reader.u8()?;
        packet.pad3 = reader.u8()?;
        packet.net_id = reader.i32()?;
        packet.secondary_id = reader.i32()?;
        packet.flags = reader.u32()?;
        packet.float1 = reader.f32()?;
        packet.int_data = reader.i32()?;
        packet.pos_x = reader.f32()?;
        packet.pos_y = reader.f32()?;
        packet.speed_x = reader.f32()?;
        packet.speed_y = reader.f32()?;
        packet.float2 = reader.f32()?;
        packet.tile_x = reader.i32()?;
        packet.tile_y = reader.i32()?;
        packet.data_size = reader.u32()?;
        Ok(packet)
    }

    pub fn encode(&self, writer: &mut Writer) {
        writer.u8(self.kind);
        writer.u8(self.pad1);
        writer.u8(self.pad2);
        writer.u8(self.pad3);
        writer.i32(self.net_id);
        writer.i32(self.secondary_id);
        writer.u32(self.flags);
        writer.f32(self.float1);
        writer.i32(self.int_data);
        writer.f32(self.pos_x);
        writer.f32(self.pos_y);
        writer.f32(self.speed_x);
        writer.f32(self.speed_y);
        writer.f32(self.float2);
        writer.i32(self.tile_x);
        writer.i32(self.tile_y);
        writer.u32(self.data_size);
    }

    #[must_use]
    pub fn is_extended(&self) -> bool {
        self.flags & FLAG_EXTENDED != 0
    }

    pub fn set_extended(&mut self, size: usize) {
        self.flags |= FLAG_EXTENDED;
        self.data_size = size as u32;
    }

    #[must_use]
    pub fn tile_to_pixel(tile: i32) -> f32 {
        tile as f32 * 32.0
    }

    #[must_use]
    pub fn pixel_to_tile(pixel: f32) -> i32 {
        (pixel / 32.0) as i32
    }
}
