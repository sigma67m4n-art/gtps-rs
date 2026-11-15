use std::path::Path;

use crate::cursor::{ProtoError, Reader, Result};

pub const HASH_SEED: u32 = 0x55555555;
pub const MAX_FORMAT_VERSION: u16 = 27;

#[derive(Debug, Clone)]
pub struct ItemDefinition {
    pub kind: u8,
    pub collision: u8,
    pub hits: u32,
    pub grow_seconds: u32,
    max_amount_offset: usize,
}

fn skip_string(reader: &mut Reader<'_>) -> Result<()> {
    let len = reader.u16()?;
    reader.skip(usize::from(len))
}

fn definition(reader: &mut Reader<'_>, version: u16, expected: u32) -> Result<ItemDefinition> {
    if reader.u32()? != expected { return Err(ProtoError::new("unordered item ID")); }
    reader.skip(2)?;
    let kind = reader.u8()?;
    reader.skip(1)?;
    skip_string(reader)?;
    skip_string(reader)?;
    reader.skip(13)?;
    let collision = reader.u8()?;
    let hits = u32::from(reader.u8()?).div_ceil(6).max(1);
    reader.skip(7)?;
    let max_amount_offset = reader.position();
    reader.skip(1)?;
    skip_string(reader)?;
    reader.skip(8)?;
    for _ in 0..4 { skip_string(reader)?; }
    reader.skip(16)?;
    let grow_seconds = reader.u32()?;
    reader.skip(4)?;
    for _ in 0..3 { skip_string(reader)?; }
    reader.skip(80)?;
    if version >= 11 { skip_string(reader)?; }
    if version >= 12 { reader.skip(13)?; }
    if version >= 13 { reader.skip(4)?; }
    if version >= 14 { reader.skip(4)?; }
    if version >= 15 { reader.skip(25)?; skip_string(reader)?; }
    if version >= 16 { skip_string(reader)?; }
    if version >= 17 { reader.skip(4)?; }
    if version >= 18 { reader.skip(4)?; }
    if version >= 19 { reader.skip(9)?; }
    if version >= 21 { reader.skip(2)?; }
    if version >= 22 { skip_string(reader)?; }
    if version >= 23 { reader.skip(4)?; }
    if version >= 24 { reader.skip(1)?; }
    if version >= 25 { skip_string(reader)?; reader.skip(4)?; }
    if version >= 26 { reader.skip(1)?; }
    if version >= 27 { skip_string(reader)?; }
    Ok(ItemDefinition { kind, collision, hits, grow_seconds, max_amount_offset })
}

#[must_use]
pub fn hash_data(data: &[u8]) -> u32 {
    let mut acc = HASH_SEED;

    for byte in data {
        acc = (acc >> 27)
            .wrapping_add(acc << 5)
            .wrapping_add(u32::from(*byte));
    }

    acc
}

#[derive(Debug, Clone)]
pub struct ItemsDat {
    raw: Vec<u8>,
    version: u16,
    count: u32,
    hash: u32,
    definitions: Vec<ItemDefinition>,
}

impl ItemsDat {
    pub fn parse(raw: Vec<u8>) -> Result<Self> {
        let hash = hash_data(&raw);
        let mut reader = Reader::new(&raw);
        let version = reader.u16()?;
        let count = reader.u32()?;
        if !(10..=MAX_FORMAT_VERSION).contains(&version) || count > 65536 {
            return Err(ProtoError::new("unsupported item database"));
        }
        let mut definitions = Vec::new();
        for id in 0..count { definitions.push(definition(&mut reader,version,id)?); }
        if !reader.is_empty() { return Err(ProtoError::new("trailing item database bytes")); }

        Ok(Self {
            raw,
            version,
            count,
            hash,
            definitions,
        })
    }

    pub fn get(&self, id: u16) -> Option<&ItemDefinition> {
        self.definitions.get(usize::from(id))
    }

    pub fn set_stack_limit(&mut self, limit: u8) {
        for definition in &self.definitions {
            if self.raw[definition.max_amount_offset] == 200 {
                self.raw[definition.max_amount_offset] = limit;
            }
        }
        self.hash = hash_data(&self.raw);
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read(path).map_err(|_| ProtoError::new("failed to read items.dat"))?;
        Self::parse(raw)
    }

    #[must_use]
    pub fn version(&self) -> u16 {
        self.version
    }

    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }

    #[must_use]
    pub fn hash(&self) -> u32 {
        self.hash
    }

    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{hash_data, HASH_SEED};

    #[test]
    fn bundled_database_validates_all_records_and_hardness() {
        let raw = include_bytes!("../../../data/items.dat").to_vec();
        let db = super::ItemsDat::parse(raw.clone()).unwrap();
        assert_eq!(db.count(),16434);
        for (id,hits) in [(2,3),(4,4),(10,10),(340,7),(324,9),(5666,5)] {
            assert_eq!(db.get(id).unwrap().hits,hits);
        }
        assert_eq!(db.get(3).unwrap().kind,19);
        assert!(super::ItemsDat::parse(raw[..raw.len()-1].to_vec()).is_err());
    }

    #[test]
    fn hash_matches_reference() {
        assert_eq!(hash_data(&[]), HASH_SEED);
        assert_eq!(hash_data(&[1]), 0xAAAA_AAAB);
    }
}
