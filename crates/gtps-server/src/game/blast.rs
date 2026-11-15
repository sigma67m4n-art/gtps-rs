pub const SMALL_BLAST: u16 = 13620;
pub const LARGE_BLAST: u16 = 13621;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blast {
    pub name: &'static str,
    pub item_id: u16,
    pub width: usize,
    pub height: usize,
}

pub const BLASTS: &[Blast] = &[
    Blast {
        name: "Small Blast",
        item_id: SMALL_BLAST,
        width: 50,
        height: 50,
    },
    Blast {
        name: "Large Blast",
        item_id: LARGE_BLAST,
        width: 200,
        height: 200,
    },
];

#[must_use]
pub fn by_item(item_id: u16) -> Option<&'static Blast> {
    BLASTS.iter().find(|blast| blast.item_id == item_id)
}

#[must_use]
pub fn world_name(sequence: u32) -> String {
    format!("BLAST{sequence}")
}

#[cfg(test)]
mod tests {
    use super::{by_item, world_name, LARGE_BLAST, SMALL_BLAST};

    #[test]
    fn blasts_match_their_sizes() {
        let small = by_item(SMALL_BLAST).expect("small blast");
        let large = by_item(LARGE_BLAST).expect("large blast");

        assert_eq!((small.width, small.height), (50, 50));
        assert_eq!((large.width, large.height), (200, 200));
        assert!(large.width > small.width);
    }

    #[test]
    fn unknown_items_have_no_blast() {
        assert!(by_item(0).is_none());
        assert!(by_item(9_999).is_none());
    }

    #[test]
    fn world_names_are_unique_and_valid() {
        let name = world_name(42);
        assert_eq!(name, "BLAST42");
        assert!(name.len() <= 24);
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
