use std::path::Path;

use crate::cursor::{ProtoError, Reader, Result};

pub const HASH_SEED: u32 = 0x55555555;
pub const MAX_FORMAT_VERSION: u16 = 26;

#[must_use]
pub fn hash_data(data: &[u8]) -> u32 {
    let mut acc = HASH_SEED;

    for byte in data {
        acc = (acc >> 27).wrapping_add(acc << 5).wrapping_add(u32::from(*byte));
    }

    acc
}

#[derive(Debug, Clone)]
pub struct ItemsDat {
    raw: Vec<u8>,
    version: u16,
    count: u32,
    hash: u32,
}

impl ItemsDat {
    pub fn parse(raw: Vec<u8>) -> Result<Self> {
        let hash = hash_data(&raw);
        let mut reader = Reader::new(&raw);
        let version = reader.u16()?;
        let count = reader.u32()?;

        Ok(Self {
            raw,
            version,
            count,
            hash,
        })
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
    fn hash_matches_reference() {
        assert_eq!(hash_data(&[]), HASH_SEED);
        assert_eq!(hash_data(&[1]), 0xAAAA_AAAB);
    }
}
