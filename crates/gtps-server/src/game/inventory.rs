use gtps_proto::cursor::{Reader, Result, Writer};
use gtps_world::items;

pub const EQUIPPED: u8 = 0x01;
pub const MAX_STACK: u8 = 200;
pub const DEFAULT_CAPACITY: u32 = 596;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub id: u16,
    pub count: u8,
    pub flags: u8,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    capacity: u32,
    items: Vec<Item>,
}

impl Default for Inventory {
    fn default() -> Self {
        let mut inventory = Self {
            capacity: DEFAULT_CAPACITY,
            items: Vec::new(),
        };
        inventory.add(items::FIST, 1);
        inventory.add(items::WRENCH, 1);
        inventory
    }
}

impl Inventory {
    #[must_use]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn set_capacity(&mut self, capacity: u32) {
        self.capacity = capacity;
    }

    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn add(&mut self, id: u16, count: u8) -> bool {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.count = item.count.saturating_add(count).min(MAX_STACK);
            return true;
        }

        if self.items.len() >= self.capacity as usize {
            return false;
        }

        self.items.push(Item { id, count, flags: 0 });
        true
    }

    pub fn remove(&mut self, id: u16, count: u8) -> bool {
        let Some(index) = self.items.iter().position(|item| item.id == id) else {
            return false;
        };

        if self.items[index].count < count {
            return false;
        }

        self.items[index].count -= count;
        if self.items[index].count == 0 {
            self.items.remove(index);
        }

        true
    }

    #[must_use]
    pub fn count_of(&self, id: u16) -> u32 {
        self.items
            .iter()
            .filter(|item| item.id == id)
            .map(|item| u32::from(item.count))
            .sum()
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::with_capacity(9 + self.items.len() * 4);
        writer.u8(1);
        writer.u32(self.capacity);
        writer.u32(self.items.len() as u32);

        for item in &self.items {
            writer.u16(item.id).u8(item.count).u8(item.flags);
        }

        writer.into_vec()
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(data);
        let _version = reader.u8()?;
        let capacity = reader.u32()?;
        let count = reader.u32()? as usize;

        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(Item {
                id: reader.u16()?,
                count: reader.u8()?,
                flags: reader.u8()?,
            });
        }

        Ok(Self { capacity, items })
    }
}

#[cfg(test)]
mod tests {
    use super::Inventory;
    use gtps_world::items;

    #[test]
    fn inventory_round_trips() {
        let mut inventory = Inventory::default();
        assert!(inventory.add(items::DIRT, 5));

        let decoded = Inventory::decode(&inventory.encode()).expect("decode");
        assert_eq!(decoded.capacity(), inventory.capacity());
        assert_eq!(decoded.count_of(items::DIRT), 5);
        assert_eq!(decoded.count_of(items::FIST), 1);
    }
}
