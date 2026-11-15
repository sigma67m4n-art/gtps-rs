use gtps_proto::cursor::{Reader, Result, Writer};
use gtps_world::items;

pub const MAX_STACK: u8 = 250;
pub const DEFAULT_CAPACITY: u32 = 596;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub id: u16,
    pub count: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn starter() -> Self {
        let mut inventory = Self::default();
        inventory.add(super::economy::highest().block_id, MAX_STACK);
        inventory.add(super::tools::RIPPER_WINGS, 1);
        inventory.add(super::tools::PICKAXE, 1);
        inventory
    }
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
        if count == 0 || count > MAX_STACK {
            return false;
        }
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            if u16::from(item.count) + u16::from(count) > u16::from(MAX_STACK) {
                return false;
            }
            item.count += count;
            return true;
        }

        if self.items.len() >= self.capacity as usize {
            return false;
        }

        self.items.push(Item {
            id,
            count,
            flags: 0,
        });
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
    pub fn can_add(&self, id: u16) -> bool {
        match self.items.iter().find(|item| item.id == id) {
            Some(item) => item.count < MAX_STACK,
            None => self.items.len() < self.capacity as usize,
        }
    }

    pub fn exchange(&mut self, source: u16, count: u8, target: u16) -> bool {
        let mut next = self.clone();
        if !next.remove(source, count) || !next.add(target, 1) {
            return false;
        }
        *self = next;
        true
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

    #[allow(dead_code)]
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
    use super::{Inventory,MAX_STACK};

    #[test]
    fn starter_and_stack_limit_round_trip() {
        let mut inventory = Inventory::starter();
        assert_eq!(inventory.count_of(542),250);
        assert_eq!(inventory.count_of(98),1);
        assert_eq!(inventory.count_of(818),1);
        assert!(!inventory.add(542,1));
        assert_eq!(Inventory::decode(&inventory.encode()).unwrap().count_of(542),250);
    }
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

    #[test]
    fn can_add_reports_capacity() {
        let mut inventory = Inventory::default();
        assert!(inventory.can_add(items::DIRT));

        inventory.set_capacity(inventory.items().len() as u32);
        assert!(!inventory.can_add(items::ROCK));
        assert!(inventory.can_add(items::FIST));
    }

    #[test]
    fn exchange_preserves_inputs_when_output_stack_is_full() {
        let mut inventory = Inventory::default();
        inventory.add(7188, 100);
        inventory.add(60004, MAX_STACK);
        assert!(!inventory.exchange(7188, 100, 60004));
        assert_eq!(inventory.count_of(7188), 100);
        inventory.remove(60004, 1);
        assert!(inventory.exchange(7188, 100, 60004));
        assert_eq!(inventory.count_of(7188), 0);
        assert_eq!(inventory.count_of(60004), 250);
        assert!(!inventory.exchange(7188, 100, 60004));
    }
}
