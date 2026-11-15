use crate::game::inventory::Inventory;

pub const DEATH_SCYTHE: u16 = 4166;
pub const PICKAXE: u16 = 98;
pub const RIPPER_WINGS: u16 = 818;
pub const RAYMAN: u16 = 5480;
pub const MAGPLANT: u16 = 5638;
pub const LEGENDARY_RAYMAN: u16 = 60000;
pub const MYTHICAL_RAYMAN: u16 = 60002;
pub const DIALOG: &str = "equipment_shop";

#[derive(Debug, Clone, Copy)]
pub struct Tool {
    pub item: u16,
    pub name: &'static str,
    pub price: u32,
    pub reach: usize,
    pub one_hit: bool,
}

pub const CATALOG: &[Tool] = &[
    Tool {
        item: RAYMAN,
        name: "Rayman's Fist",
        price: 50_000,
        reach: 3,
        one_hit: false,
    },
    Tool {
        item: DEATH_SCYTHE,
        name: "Death's Scythe",
        price: 250_000,
        reach: 1,
        one_hit: true,
    },
    Tool {
        item: LEGENDARY_RAYMAN,
        name: "Legendary Infinity Rayman's Fist",
        price: 25_000_000,
        reach: 7,
        one_hit: true,
    },
    Tool {
        item: MYTHICAL_RAYMAN,
        name: "Mythical Infinity Rayman's Fist",
        price: 750_000_000,
        reach: 10,
        one_hit: true,
    },
    Tool {
        item: MAGPLANT,
        name: "MAGPLANT 5000",
        price: 75_000,
        reach: 0,
        one_hit: false,
    },
];

pub fn by_item(item: u16) -> Option<&'static Tool> {
    CATALOG.iter().find(|tool| tool.item == item)
}

pub fn shop_only(item: u16) -> bool {
    CATALOG
        .iter()
        .any(|tool| item == tool.item || item == tool.item + 1)
}

pub fn equipped(inventory: &Inventory, item: u16) -> Option<&'static Tool> {
    by_item(item).filter(|tool| tool.reach > 0 && inventory.count_of(item) > 0)
}

pub fn reach(inventory: &Inventory, item: u16) -> usize {
    equipped(inventory, item).map_or(1, |tool| tool.reach)
}

pub fn hits(inventory: &Inventory, item: u16, normal: u32) -> u32 {
    if equipped(inventory, item).is_some_and(|tool| tool.one_hit) {
        1
    } else if item == PICKAXE && inventory.count_of(item) > 0 {
        (normal * 5).div_ceil(6).max(1)
    } else {
        normal.max(1)
    }
}

pub fn purchase(inventory: &mut Inventory, gems: &mut u32, item: u16) -> bool {
    let Some(tool) = by_item(item) else {
        return false;
    };
    if *gems < tool.price || !inventory.add(item, 1) {
        return false;
    }
    *gems -= tool.price;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_owned_equipped_tools_change_hits_and_reach() {
        let mut inventory = Inventory::default();
        assert_eq!(reach(&inventory, MYTHICAL_RAYMAN), 1);
        assert_eq!(hits(&inventory, MYTHICAL_RAYMAN, 6), 6);
        for (item, distance, count) in [
            (DEATH_SCYTHE, 1, 1),
            (RAYMAN, 3, 6),
            (LEGENDARY_RAYMAN, 7, 1),
            (MYTHICAL_RAYMAN, 10, 1),
        ] {
            assert!(inventory.add(item, 1));
            assert_eq!(reach(&inventory, item), distance);
            assert_eq!(hits(&inventory, item, 6), count);
        }
        assert_eq!(reach(&inventory, 0), 1);
        assert_eq!(hits(&inventory, 0, 6), 6);
    }

    #[test]
    fn purchase_is_atomic_on_insufficient_gems_and_full_stack() {
        let mut inventory = Inventory::default();
        let mut gems = 749_999_999;
        assert!(!purchase(&mut inventory, &mut gems, MYTHICAL_RAYMAN));
        assert_eq!(gems, 749_999_999);
        gems += 1;
        assert!(purchase(&mut inventory, &mut gems, MYTHICAL_RAYMAN));
        assert_eq!(gems, 0);
        assert_eq!(inventory.count_of(MYTHICAL_RAYMAN), 1);
        assert!(inventory.add(DEATH_SCYTHE, 250));
        gems = 250_000;
        assert!(!purchase(&mut inventory, &mut gems, DEATH_SCYTHE));
        assert_eq!(gems, 250_000);
        assert_eq!(inventory.count_of(DEATH_SCYTHE), 250);
    }

    #[test]
    fn tools_and_their_seeds_are_shop_only() {
        for tool in CATALOG {
            assert!(shop_only(tool.item));
            assert!(shop_only(tool.item + 1));
        }
        assert!(!shop_only(2));
    }
}
