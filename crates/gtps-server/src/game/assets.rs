use gtps_proto::items::{hash_data, CustomItem};
use gtps_proto::ItemsDat;

pub const TEXTURE: &[u8] = include_bytes!("../../../../assets/custom-icons.rttex");

pub fn install(items: &mut ItemsDat) -> gtps_proto::cursor::Result<()> {
    let hash = hash_data(TEXTURE);
    let texture = |x| Some(("custom-icons.rttex", hash, x, 0));
    items.append_custom(&[
        CustomItem {
            id: super::tools::LEGENDARY_RAYMAN,
            template: super::tools::PICKAXE,
            name: "Legendary Infinity Rayman's Fist",
            texture: texture(0),
        },
        CustomItem {
            id: super::tools::MYTHICAL_RAYMAN,
            template: super::tools::PICKAXE,
            name: "Mythical Infinity Rayman's Fist",
            texture: texture(1),
        },
        CustomItem {
            id: 60004,
            template: 7188,
            name: "White Gem Lock",
            texture: texture(2),
        },
        CustomItem {
            id: super::blast::SMALL_BLAST,
            template: 1402,
            name: "Small Blast",
            texture: None,
        },
        CustomItem {
            id: super::blast::LARGE_BLAST,
            template: 1402,
            name: "Large Blast",
            texture: None,
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_database_round_trip_preserves_vanilla_and_rejects_reserved_gaps() {
        let raw = include_bytes!("../../../../data/items.dat");
        let mut items = ItemsDat::parse(raw.to_vec()).unwrap();
        install(&mut items).unwrap();
        items.set_stack_limit(250);
        let parsed = ItemsDat::parse(items.raw().to_vec()).unwrap();
        assert_eq!(parsed.count(), 60011);
        assert_eq!(parsed.get(60000).unwrap().kind, 20);
        assert_eq!(
            parsed.get(60004).unwrap().kind,
            parsed.get(7188).unwrap().kind
        );
        assert!(!parsed.get(60006).unwrap().available);
        assert_eq!(parsed.get(12600).unwrap().kind, 8);
        assert!(parsed.get(60010).unwrap().available);
    }
}
