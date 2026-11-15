pub const TELEPHONE_ITEM: u16 = 53785;
pub const RAYMAN_FIST: u16 = 13634;
pub const FIST_RANGE: usize = 1;
pub const RAYMAN_FIST_RANGE: usize = 3;

pub const MILLION: u64 = 1_000_000;
pub const TAX_PER_MILLION: u64 = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    World,
    Diamond,
    BlueGem,
    Emerald,
    Royal,
}

impl LockKind {
    #[must_use]
    pub fn sale_button(self) -> &'static str {
        match self {
            Self::Diamond => "buy_diamond",
            Self::BlueGem => "buy_bluegem",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockSpec {
    pub kind: LockKind,
    pub name: &'static str,
    pub item_id: u16,
    pub magplant_stock: u32,
    pub gems_percent: u32,
    pub shop_price: u64,
}

pub const LOCKS: &[LockSpec] = &[
    LockSpec {
        kind: LockKind::World,
        name: "World Lock",
        item_id: 1796,
        magplant_stock: 5_000,
        gems_percent: 100,
        shop_price: 0,
    },
    LockSpec {
        kind: LockKind::Diamond,
        name: "Diamond Lock",
        item_id: 7188,
        magplant_stock: 15_000,
        gems_percent: 100,
        shop_price: 100_000,
    },
    LockSpec {
        kind: LockKind::BlueGem,
        name: "Blue Gem Lock",
        item_id: 12886,
        magplant_stock: 50_000,
        gems_percent: 100,
        shop_price: 10_000_000,
    },
    LockSpec {
        kind: LockKind::Emerald,
        name: "Emerald Lock",
        item_id: 13254,
        magplant_stock: 100_000,
        gems_percent: 150,
        shop_price: 0,
    },
    LockSpec {
        kind: LockKind::Royal,
        name: "Royal Lock",
        item_id: 8482,
        magplant_stock: 200_000,
        gems_percent: 200,
        shop_price: 0,
    },
];

#[must_use]
pub fn by_kind(kind: LockKind) -> &'static LockSpec {
    LOCKS
        .iter()
        .find(|spec| spec.kind == kind)
        .expect("lock kind is always present")
}

#[must_use]
pub fn by_item(item_id: u16) -> Option<&'static LockSpec> {
    LOCKS.iter().find(|spec| spec.item_id == item_id)
}

#[must_use]
pub fn magplant_stock(lock_item: u16) -> u32 {
    by_item(lock_item).map_or(0, |spec| spec.magplant_stock)
}

#[must_use]
pub fn multiplier_percent(lock_item: u16) -> u32 {
    by_item(lock_item).map_or(100, |spec| spec.gems_percent)
}

#[must_use]
pub fn tax(price: u64) -> u64 {
    (price / MILLION) * TAX_PER_MILLION
}

#[must_use]
pub fn total_with_tax(price: u64) -> u64 {
    price + tax(price)
}

pub fn for_sale() -> impl Iterator<Item = &'static LockSpec> {
    LOCKS.iter().filter(|spec| spec.shop_price > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Magplant {
    pub tile: usize,
    pub lock_item: u16,
    pub stock: u32,
}

impl Magplant {
    #[must_use]
    pub fn new(tile: usize, lock_item: u16) -> Self {
        Self {
            tile,
            lock_item,
            stock: magplant_stock(lock_item),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stock == 0
    }

    pub fn take(&mut self, amount: u32) -> u32 {
        let taken = self.stock.min(amount);
        self.stock -= taken;
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::{
        by_item, by_kind, magplant_stock, multiplier_percent, tax, total_with_tax, LockKind,
        Magplant, MILLION,
    };

    #[test]
    fn magplant_stock_per_lock() {
        assert_eq!(magplant_stock(by_kind(LockKind::World).item_id), 5_000);
        assert_eq!(magplant_stock(by_kind(LockKind::Diamond).item_id), 15_000);
        assert_eq!(magplant_stock(by_kind(LockKind::BlueGem).item_id), 50_000);
        assert_eq!(magplant_stock(by_kind(LockKind::Emerald).item_id), 100_000);
        assert_eq!(magplant_stock(by_kind(LockKind::Royal).item_id), 200_000);
    }

    #[test]
    fn emerald_and_royal_boost_gems() {
        assert_eq!(multiplier_percent(by_kind(LockKind::Emerald).item_id), 150);
        assert_eq!(multiplier_percent(by_kind(LockKind::Royal).item_id), 200);
        assert_eq!(multiplier_percent(by_kind(LockKind::World).item_id), 100);
    }

    #[test]
    fn tax_is_five_percent_per_million() {
        assert_eq!(tax(100_000), 0);
        assert_eq!(tax(MILLION), 50_000);
        assert_eq!(tax(10 * MILLION), 500_000);
        assert_eq!(total_with_tax(10 * MILLION), 10_500_000);
    }

    #[test]
    fn magplant_drains_and_scans() {
        let mut magplant = Magplant::new(42, by_kind(LockKind::Diamond).item_id);
        assert_eq!(magplant.stock, 15_000);
        assert_eq!(magplant.take(500), 500);
        assert!(!magplant.is_empty());
        assert_eq!(magplant.take(20_000), 14_500);
        assert!(magplant.is_empty());
    }

    #[test]
    fn only_diamond_and_blue_gem_are_sold() {
        let sold: Vec<_> = super::for_sale().map(|spec| spec.kind).collect();
        assert_eq!(sold, vec![LockKind::Diamond, LockKind::BlueGem]);
        assert_eq!(by_item(12886).map(|spec| spec.kind), Some(LockKind::BlueGem));
        assert_eq!(sold[0].sale_button(), "buy_diamond");
        assert_eq!(sold[1].sale_button(), "buy_bluegem");
        assert!(by_kind(LockKind::Royal).shop_price == 0);
    }
}
