#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yield {
    Harvest,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Farmable {
    pub name: &'static str,
    pub block_id: u16,
    pub seed_id: u16,
    pub break_gems: u32,
    pub hits: u32,
    pub grow_seconds: u32,
}

pub const GEMS_MULTIPLIER_BASE: u32 = 100;
pub const HARVEST_PERCENT: u32 = 65;
pub const FARMABLES: &[Farmable] = &[Farmable {
    name: "Pot O' Gold",
    block_id: 542,
    seed_id: 543,
    break_gems: 240,
    hits: 6,
    grow_seconds: 240,
}];

pub fn table() -> &'static [Farmable] {
    FARMABLES
}
pub fn highest() -> &'static Farmable {
    &FARMABLES[0]
}
pub fn harvest_gems(farmable: &Farmable) -> u32 {
    farmable.break_gems * HARVEST_PERCENT / GEMS_MULTIPLIER_BASE
}
pub fn payout(farmable: &Farmable, kind: Yield, multiplier_percent: u32, _roll: u32) -> u32 {
    let base = match kind {
        Yield::Break => farmable.break_gems,
        Yield::Harvest => harvest_gems(farmable),
    };
    base * multiplier_percent / GEMS_MULTIPLIER_BASE
}
pub fn cycle_gems(farmable: &Farmable, multiplier_percent: u32, roll: u32) -> u32 {
    payout(farmable, Yield::Break, multiplier_percent, roll)
}

pub const GEMS_PER_GROWTOKEN: u64 = 100_000;
pub const GEMS_PER_SPRAY_TOKEN: u64 = 1_000_000;
pub const SPRAY_TOKEN_ICON: u16 = 4212;
pub const ULTRA_WORLD_SPRAY: u16 = 12600;
pub const ULTRA_WORLD_SPRAY_PRICE: u32 = 5;

fn exchange(credits: u64, rate: u64) -> (u32, u64) {
    let tokens = u32::try_from(credits / rate).unwrap_or(u32::MAX);
    (tokens, credits - u64::from(tokens) * rate)
}
pub fn exchange_tokens(credits: u64) -> (u32, u64) {
    exchange(credits, GEMS_PER_GROWTOKEN)
}
pub fn exchange_spray_tokens(credits: u64) -> (u32, u64) {
    exchange(credits, GEMS_PER_SPRAY_TOKEN)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pot_is_the_only_farmable_with_stable_break_and_lower_harvest() {
        assert_eq!(table().len(), 1);
        let pot = highest();
        assert_eq!((pot.block_id, pot.seed_id, pot.hits), (542, 543, 6));
        for roll in 0..64 {
            assert_eq!(payout(pot, Yield::Break, 100, roll), 240);
            assert_eq!(payout(pot, Yield::Harvest, 100, roll), 156);
            assert_eq!(cycle_gems(pot, 225, roll), 540);
        }
    }
    #[test]
    fn exchanges_preserve_remainders() {
        assert_eq!(exchange_tokens(99_999), (0, 99_999));
        assert_eq!(exchange_tokens(300_042), (3, 42));
        assert_eq!(exchange_spray_tokens(2_000_005), (2, 5));
    }
}
