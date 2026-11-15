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
    offset: usize,
    end: usize,
    texture_offset: usize,
    properties_offset: usize,
    pub available: bool,
}

pub struct CustomItem<'a> {
    pub id: u16,
    pub template: u16,
    pub name: &'a str,
    pub texture: Option<(&'a str, u32, u8, u8)>,
}

fn skip_string(reader: &mut Reader<'_>) -> Result<()> {
    let len = reader.u16()?;
    reader.skip(usize::from(len))
}

fn definition(reader: &mut Reader<'_>, version: u16, expected: u32) -> Result<ItemDefinition> {
    let offset = reader.position();
    if reader.u32()? != expected {
        return Err(ProtoError::new("unordered item ID"));
    }
    reader.skip(2)?;
    let kind = reader.u8()?;
    reader.skip(1)?;
    let name_length = reader.u16()?;
    reader.skip(usize::from(name_length))?;
    let texture_offset = reader.position();
    skip_string(reader)?;
    let properties_offset = reader.position();
    reader.skip(13)?;
    let collision = reader.u8()?;
    let hits = u32::from(reader.u8()?).div_ceil(6).max(1);
    reader.skip(7)?;
    let max_amount_offset = reader.position();
    reader.skip(1)?;
    skip_string(reader)?;
    reader.skip(8)?;
    for _ in 0..4 {
        skip_string(reader)?;
    }
    reader.skip(16)?;
    let grow_seconds = reader.u32()?;
    reader.skip(4)?;
    for _ in 0..3 {
        skip_string(reader)?;
    }
    reader.skip(80)?;
    if version >= 11 {
        skip_string(reader)?;
    }
    if version >= 12 {
        reader.skip(13)?;
    }
    if version >= 13 {
        reader.skip(4)?;
    }
    if version >= 14 {
        reader.skip(4)?;
    }
    if version >= 15 {
        reader.skip(25)?;
        skip_string(reader)?;
    }
    if version >= 16 {
        skip_string(reader)?;
    }
    if version >= 17 {
        reader.skip(4)?;
    }
    if version >= 18 {
        reader.skip(4)?;
    }
    if version >= 19 {
        reader.skip(9)?;
    }
    if version >= 21 {
        reader.skip(2)?;
    }
    if version >= 22 {
        skip_string(reader)?;
    }
    if version >= 23 {
        reader.skip(4)?;
    }
    if version >= 24 {
        reader.skip(1)?;
    }
    if version >= 25 {
        skip_string(reader)?;
        reader.skip(4)?;
    }
    if version >= 26 {
        reader.skip(1)?;
    }
    if version >= 27 {
        skip_string(reader)?;
    }
    Ok(ItemDefinition {
        kind,
        collision,
        hits,
        grow_seconds,
        max_amount_offset,
        offset,
        end: reader.position(),
        texture_offset,
        properties_offset,
        available: name_length != 0,
    })
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
    pub fn append_custom(&mut self, items: &[CustomItem<'_>]) -> Result<()> {
        let mut raw = self.raw.clone();
        let mut count = self.count;
        for item in items {
            if u32::from(item.id) < count || item.name.len() > u16::MAX as usize {
                return Err(ProtoError::new("invalid custom item ID or name"));
            }
            let template = self
                .get(item.template)
                .ok_or_else(|| ProtoError::new("missing item template"))?;
            while count < u32::from(item.id) {
                let empty = self
                    .get(0)
                    .ok_or_else(|| ProtoError::new("missing empty item template"))?;
                let mut record = self.record(empty, count, "", None)?;
                record[6] = 0;
                raw.extend_from_slice(&record);
                count += 1;
            }
            raw.extend_from_slice(&self.record(template, count, item.name, item.texture)?);
            count += 1;
        }
        raw[2..6].copy_from_slice(&count.to_le_bytes());
        *self = Self::parse(raw)?;
        Ok(())
    }

    fn record(
        &self,
        template: &ItemDefinition,
        id: u32,
        name: &str,
        texture: Option<(&str, u32, u8, u8)>,
    ) -> Result<Vec<u8>> {
        let mut record = self.raw[template.offset..template.offset + 8].to_vec();
        record[..4].copy_from_slice(&id.to_le_bytes());
        let key = b"PBG892FXX982ABC*";
        record.extend_from_slice(&(name.len() as u16).to_le_bytes());
        record.extend(
            name.bytes()
                .enumerate()
                .map(|(index, byte)| byte ^ key[(id as usize + index) % key.len()]),
        );
        if let Some((path, hash, x, y)) = texture {
            let length =
                u16::try_from(path.len()).map_err(|_| ProtoError::new("texture path too long"))?;
            record.extend_from_slice(&length.to_le_bytes());
            record.extend_from_slice(path.as_bytes());
            let properties = record.len();
            record.extend_from_slice(&self.raw[template.properties_offset..template.end]);
            record[properties..properties + 4].copy_from_slice(&hash.to_le_bytes());
            record[properties + 4] = 0;
            record[properties + 9] = x;
            record[properties + 10] = y;
            record[properties + 11] = 1;
        } else {
            record.extend_from_slice(&self.raw[template.texture_offset..template.end]);
        }
        Ok(record)
    }

    pub fn parse(raw: Vec<u8>) -> Result<Self> {
        let hash = hash_data(&raw);
        let mut reader = Reader::new(&raw);
        let version = reader.u16()?;
        let count = reader.u32()?;
        if !(10..=MAX_FORMAT_VERSION).contains(&version) || count > 65536 {
            return Err(ProtoError::new("unsupported item database"));
        }
        let mut definitions = Vec::new();
        for id in 0..count {
            definitions.push(definition(&mut reader, version, id)?);
        }
        if !reader.is_empty() {
            return Err(ProtoError::new("trailing item database bytes"));
        }

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

    pub fn configure_farmable(&mut self, block: u16, hits: u8) {
        if let Some(definition) = self.definitions.get_mut(usize::from(block)) {
            definition.kind = 17;
            definition.hits = u32::from(hits);
            self.raw[definition.offset + 6] = 17;
            self.raw[definition.max_amount_offset - 8] = hits.saturating_mul(6);
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
        assert_eq!(db.count(), 16434);
        for (id, hits) in [(2, 3), (4, 4), (10, 10), (340, 7), (324, 9), (5666, 5)] {
            assert_eq!(db.get(id).unwrap().hits, hits);
        }
        assert_eq!(db.get(3).unwrap().kind, 19);
        assert!(super::ItemsDat::parse(raw[..raw.len() - 1].to_vec()).is_err());
    }

    #[test]
    fn served_database_preserves_layout_after_stack_and_farm_changes() {
        let mut db =
            super::ItemsDat::parse(include_bytes!("../../../data/items.dat").to_vec()).unwrap();
        let old_hash = db.hash();
        db.set_stack_limit(250);
        db.configure_farmable(542, 6);
        let reparsed = super::ItemsDat::parse(db.raw().to_vec()).unwrap();
        assert_ne!(old_hash, reparsed.hash());
        let pot = reparsed.get(542).unwrap();
        assert_eq!((pot.kind, pot.hits), (17, 6));
        assert_eq!(reparsed.raw()[pot.max_amount_offset], 250);
        assert_eq!(reparsed.get(4).unwrap().hits, 4);
    }

    #[test]
    fn hash_matches_reference() {
        assert_eq!(hash_data(&[]), HASH_SEED);
        assert_eq!(hash_data(&[1]), 0xAAAA_AAAB);
    }

    #[test]
    fn custom_records_rekey_names_and_encode_texture_coordinates() {
        let mut db =
            super::ItemsDat::parse(include_bytes!("../../../data/items.dat").to_vec()).unwrap();
        let original = db.raw.clone();
        let name = "White Gem Lock";
        db.append_custom(&[super::CustomItem {
            id: 60004,
            template: 7188,
            name,
            texture: Some(("custom-icons.rttex", 123456, 2, 0)),
        }])
        .unwrap();
        assert_eq!(&db.raw[6..original.len()], &original[6..]);
        let definition = db.get(60004).unwrap();
        let encrypted = &db.raw[definition.offset + 10..definition.texture_offset];
        let key = b"PBG892FXX982ABC*";
        let decoded: Vec<_> = encrypted
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ key[(60004 + index) % key.len()])
            .collect();
        assert_eq!(decoded, name.as_bytes());
        let properties = definition.properties_offset;
        assert_eq!(
            &db.raw[properties..properties + 4],
            &123456u32.to_le_bytes()
        );
        assert_eq!(&db.raw[properties + 9..properties + 12], &[2, 0, 1]);
        let snapshot = db.raw.clone();
        assert!(db
            .append_custom(&[super::CustomItem {
                id: 10,
                template: 2,
                name: "Invalid",
                texture: None
            }])
            .is_err());
        assert_eq!(db.raw, snapshot);
    }
}
