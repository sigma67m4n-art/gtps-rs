use gtps_proto::cursor::{ProtoError, Reader, Result, Writer};

use crate::tile::Tile;
use crate::world::{GroundObject, World, FORMAT_VERSION, HEIGHT, TILE_COUNT, WIDTH};

pub const TILE_HEADER_PADDING: usize = 7;
pub const OBJECT_HEADER_PADDING: usize = 12;

pub fn write_tile(writer: &mut Writer, tile: &Tile) {
    writer.u16(tile.foreground);
    writer.u16(tile.background);
    writer.u16(tile.parent);
    let [low, high] = tile.packed_flags();
    writer.u8(low).u8(high);
}

pub fn read_tile(reader: &mut Reader<'_>) -> Result<Tile> {
    let foreground = reader.u16()?;
    let background = reader.u16()?;
    let parent = reader.u16()?;
    let low = reader.u8()?;
    let high = reader.u8()?;
    let tile = Tile {
        foreground,
        background,
        parent,
        flags: Tile::from_packed_flags(low, high),
    };

    if tile.has_extra() {
        return Err(ProtoError::new("tile extra data is not supported"));
    }

    Ok(tile)
}

#[must_use]
pub fn encode_tile(tile: &Tile) -> Vec<u8> {
    let mut writer = Writer::with_capacity(8);
    write_tile(&mut writer, tile);
    writer.into_vec()
}

pub fn decode_tile(data: &[u8]) -> Result<Tile> {
    read_tile(&mut Reader::new(data))
}

pub fn write_object(writer: &mut Writer, object: &GroundObject) {
    writer.u16(object.item_id);
    writer.f32(object.pos_x);
    writer.f32(object.pos_y);
    writer.u16(object.count);
    writer.u32(object.uid);
}

pub fn read_object(reader: &mut Reader<'_>) -> Result<GroundObject> {
    Ok(GroundObject {
        item_id: reader.u16()?,
        pos_x: reader.f32()?,
        pos_y: reader.f32()?,
        count: reader.u16()?,
        uid: reader.u32()?,
    })
}

#[must_use]
pub fn encode(world: &World) -> Vec<u8> {
    let mut writer = Writer::with_capacity(
        17 + world.name.len() + world.tiles.len() * 8 + world.objects.len() * 16,
    );

    writer.u16(FORMAT_VERSION);
    writer.u32(0);
    writer.str16(&world.name);
    writer.u32(WIDTH as u32);
    writer.u32(HEIGHT as u32);
    writer.u16(world.tiles.len() as u16);
    writer.zeros(TILE_HEADER_PADDING);

    for tile in &world.tiles {
        write_tile(&mut writer, tile);
    }

    writer.zeros(OBJECT_HEADER_PADDING);
    writer.u32(world.next_object_uid);
    writer.u32(world.objects.len() as u32);

    for object in &world.objects {
        write_object(&mut writer, object);
    }

    writer.u16(world.base_weather);
    writer.u16(world.current_weather);
    writer.u32(world.owner_user_id);
    writer.u32(world.flags);
    writer.i32(world.main_lock);

    writer.into_vec()
}

pub fn decode(data: &[u8]) -> Result<World> {
    let mut reader = Reader::new(data);

    let version = reader.u16()?;
    if version != FORMAT_VERSION {
        return Err(ProtoError::new("unsupported world format version"));
    }

    let _reserved = reader.u32()?;
    let name = reader.str16()?.to_string();

    if reader.u32()? as usize != WIDTH || reader.u32()? as usize != HEIGHT {
        return Err(ProtoError::new("unexpected world dimensions"));
    }

    let tile_count = reader.u16()? as usize;
    if tile_count != TILE_COUNT {
        return Err(ProtoError::new("unexpected tile count"));
    }

    reader.skip(TILE_HEADER_PADDING)?;

    let mut tiles = Vec::with_capacity(tile_count);
    for _ in 0..tile_count {
        tiles.push(read_tile(&mut reader)?);
    }

    reader.skip(OBJECT_HEADER_PADDING)?;
    let next_object_uid = reader.u32()?;
    let object_count = reader.u32()? as usize;

    let mut objects = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        objects.push(read_object(&mut reader)?);
    }

    Ok(World {
        name,
        tiles,
        objects,
        next_object_uid,
        base_weather: reader.u16()?,
        current_weather: reader.u16()?,
        owner_user_id: reader.u32()?,
        flags: reader.u32()?,
        main_lock: reader.i32()?,
    })
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};
    use crate::generate::default_world;

    #[test]
    fn world_round_trips() {
        let world = default_world("TESTWORLD");
        let encoded = encode(&world);
        let decoded = decode(&encoded).expect("decode");

        assert_eq!(decoded.name, world.name);
        assert_eq!(decoded.tiles, world.tiles);
        assert_eq!(decoded.flags, world.flags);
        assert_eq!(decoded.main_lock, world.main_lock);
    }
}
