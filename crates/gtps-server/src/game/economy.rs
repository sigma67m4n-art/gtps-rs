#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Low,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yield {
    Harvest,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Farmable {
    pub name: &'static str,
    pub block_id: u16,
    pub seed_id: u16,
    pub break_gems: u32,
    pub hits: u32,
    pub tier: Tier,
    pub stability: u32,
    pub grow_seconds: u32,
    pub rarity: u16,
}

pub const GEMS_MULTIPLIER_BASE: u32 = 100;
pub const HARVEST_PERCENT: u32 = 65;

pub const FARMABLES: &[Farmable] = &[
    Farmable {
        name: "Pot O' Gold",
        block_id: 10340,
        seed_id: 10341,
        break_gems: 240,
        hits: 6,
        tier: Tier::High,
        stability: 0,
        grow_seconds: 240,
        rarity: 999,
    },
    Farmable {
        name: "Lava",
        block_id: 10,
        seed_id: 11,
        break_gems: 160,
        hits: 1,
        tier: Tier::High,
        stability: 5,
        grow_seconds: 180,
        rarity: 500,
    },
    Farmable {
        name: "Sand",
        block_id: 20,
        seed_id: 21,
        break_gems: 125,
        hits: 1,
        tier: Tier::High,
        stability: 8,
        grow_seconds: 150,
        rarity: 400,
    },
    Farmable {
        name: "Wood Block",
        block_id: 22,
        seed_id: 23,
        break_gems: 95,
        hits: 1,
        tier: Tier::High,
        stability: 10,
        grow_seconds: 120,
        rarity: 300,
    },
    Farmable {
        name: "Brick",
        block_id: 26,
        seed_id: 27,
        break_gems: 70,
        hits: 1,
        tier: Tier::High,
        stability: 12,
        grow_seconds: 100,
        rarity: 200,
    },
    Farmable {
        name: "Glass Block",
        block_id: 24,
        seed_id: 25,
        break_gems: 50,
        hits: 1,
        tier: Tier::High,
        stability: 15,
        grow_seconds: 90,
        rarity: 150,
    },
    Farmable {
        name: "Dirt",
        block_id: 2,
        seed_id: 3,
        break_gems: 15,
        hits: 1,
        tier: Tier::Low,
        stability: 20,
        grow_seconds: 60,
        rarity: 100,
    },
    Farmable {
        name: "Rock",
        block_id: 4,
        seed_id: 5,
        break_gems: 12,
        hits: 1,
        tier: Tier::Low,
        stability: 20,
        grow_seconds: 60,
        rarity: 80,
    },
    Farmable {
        name: "Cave Background",
        block_id: 14,
        seed_id: 15,
        break_gems: 5,
        hits: 1,
        tier: Tier::Low,
        stability: 25,
        grow_seconds: 30,
        rarity: 30,
    },
];

#[must_use]
pub fn table() -> &'static [Farmable] {
    FARMABLES
}

#[must_use]
pub fn highest() -> &'static Farmable {
    &FARMABLES[0]
}

#[must_use]
pub fn harvest_gems(farmable: &Farmable) -> u32 {
    match farmable.tier {
        Tier::Low => farmable.break_gems,
        Tier::High => farmable.break_gems * HARVEST_PERCENT / GEMS_MULTIPLIER_BASE,
    }
}

#[must_use]
pub fn base_gems(farmable: &Farmable, kind: Yield) -> u32 {
    match kind {
        Yield::Break => farmable.break_gems,
        Yield::Harvest => harvest_gems(farmable),
    }
}

#[must_use]
pub fn payout(farmable: &Farmable, kind: Yield, multiplier_percent: u32, roll: u32) -> u32 {
    let base = base_gems(farmable, kind) * multiplier_percent / GEMS_MULTIPLIER_BASE;

    if farmable.stability == 0 {
        return base;
    }

    let spread = (base * farmable.stability / GEMS_MULTIPLIER_BASE).max(1);
    let delta = roll % (spread * 2 + 1);

    base - spread + delta
}

#[must_use]
pub fn cycle_gems(farmable: &Farmable, multiplier_percent: u32, roll: u32) -> u32 {
    payout(farmable, Yield::Break, multiplier_percent, roll)
}

pub const GEMS_PER_GROWTOKEN: u64 = 100_000;
pub const GEMS_PER_SPRAY_TOKEN: u64 = 1_000_000;

pub const SPRAY_TOKEN_ICON: u16 = 4212;
pub const ULTRA_WORLD_SPRAY: u16 = 13683;
pub const ULTRA_WORLD_SPRAY_PRICE: u32 = 5;

fn exchange(credits: u64, rate: u64) -> (u32, u64) {
    let rate = rate.max(1);
    let tokens = u32::try_from(credits / rate).unwrap_or(u32::MAX);

    let spent = u64::from(tokens) * rate;
    (tokens, credits - spent)
}

#[must_use]
pub fn exchange_tokens(credits: u64) -> (u32, u64) {
    exchange(credits, GEMS_PER_GROWTOKEN)
}

#[must_use]
pub fn exchange_spray_tokens(credits: u64) -> (u32, u64) {
    exchange(credits, GEMS_PER_SPRAY_TOKEN)
}

#[cfg(test)]
mod tests {
    use super::{cycle_gems, exchange_tokens, harvest_gems, highest, payout, table, Tier, Yield};

    #[test]
    fn table_is_sorted_by_break_gems() {
        let mut previous = u32::MAX;

        for farmable in table() {
            assert!(
                farmable.break_gems <= previous,
                "{} is out of order",
                farmable.name
            );
            previous = farmable.break_gems;
        }

        assert_eq!(highest().name, "Pot O' Gold");
        assert_eq!(
            highest().break_gems,
            table().iter().map(|f| f.break_gems).max().unwrap()
        );
    }

    #[test]
    fn high_tier_harvest_is_sixty_five_percent() {
        for farmable in table().iter().filter(|f| f.tier == Tier::High) {
            assert_eq!(
                harvest_gems(farmable),
                farmable.break_gems * super::HARVEST_PERCENT / super::GEMS_MULTIPLIER_BASE,
                "{} harvest",
                farmable.name
            );
            assert!(harvest_gems(farmable) < farmable.break_gems);
        }
    }

    #[test]
    fn low_tier_is_not_adjusted() {
        for farmable in table().iter().filter(|f| f.tier == Tier::Low) {
            assert_eq!(harvest_gems(farmable), farmable.break_gems);
        }
    }

    #[test]
    fn break_beats_harvest_for_high_tier() {
        let pot = highest();
        assert!(payout(pot, Yield::Break, 100, 0) > payout(pot, Yield::Harvest, 100, 0));
    }

    #[test]
    fn pot_o_gold_block_takes_five_to_six_hits() {
        let pot = highest();
        assert!((5..=6).contains(&pot.hits));
        assert_eq!(cycle_gems(pot, 100, 0), pot.break_gems);
    }

    #[test]
    fn single_hit_farmables_cycle_equals_break() {
        for farmable in table().iter().filter(|f| f.hits == 1) {
            for roll in 0..128 {
                assert_eq!(
                    cycle_gems(farmable, 100, roll),
                    payout(farmable, Yield::Break, 100, roll),
                    "{} cycle",
                    farmable.name
                );
            }
        }
    }

    #[test]
    fn pot_o_gold_is_stable() {
        let pot = highest();

        for roll in 0..64 {
            assert_eq!(payout(pot, Yield::Break, 100, roll), pot.break_gems);
            assert_eq!(payout(pot, Yield::Harvest, 100, roll), harvest_gems(pot));
        }

        assert_eq!(payout(pot, Yield::Break, 200, 7), pot.break_gems * 2);
    }

    #[test]
    fn unstable_farmables_stay_within_band() {
        let rock = table().iter().find(|f| f.name == "Rock").expect("rock");
        let low = rock.break_gems - rock.break_gems * rock.stability / 100;
        let high = rock.break_gems + rock.break_gems * rock.stability / 100;

        for roll in 0..1024 {
            let value = payout(rock, Yield::Break, 100, roll);
            assert!((low..=high).contains(&value), "roll {roll} -> {value}");
        }
    }

    #[test]
    fn growtokens_stay_scarce() {
        let rate = super::GEMS_PER_GROWTOKEN;
        assert!(rate >= 100_000);
        assert_eq!(exchange_tokens(rate - 1), (0, rate - 1));
        assert_eq!(exchange_tokens(rate), (1, 0));
        assert_eq!(exchange_tokens(rate * 3 + 42), (3, 42));
    }

    #[test]
    fn spray_tokens_are_rarer_than_growtokens() {
        use super::exchange_spray_tokens;

        assert!(super::GEMS_PER_SPRAY_TOKEN > super::GEMS_PER_GROWTOKEN);

        let credits = super::GEMS_PER_GROWTOKEN * 12;
        let (grow, _) = exchange_tokens(credits);
        let (spray, _) = exchange_spray_tokens(credits);
        assert!(spray < grow);

        assert_eq!(exchange_spray_tokens(super::GEMS_PER_SPRAY_TOKEN), (1, 0));
        assert_eq!(
            exchange_spray_tokens(super::GEMS_PER_SPRAY_TOKEN * 2 + 5),
            (2, 5)
        );
    }

    #[test]
    fn ultra_world_spray_has_a_price() {
        assert!(super::ULTRA_WORLD_SPRAY_PRICE > 0);
        assert_ne!(super::ULTRA_WORLD_SPRAY, 0);
        assert_ne!(super::SPRAY_TOKEN_ICON, 0);
    }
}
