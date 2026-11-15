pub const FLAG_HAS_EXTRA: u16 = 0x0001;
pub const FLAG_LOCKED: u16 = 0x0002;
pub const FLAG_HAS_PARENT: u16 = 0x0004;
pub const FLAG_SPLICED: u16 = 0x0008;
pub const FLAG_SEEDLING: u16 = 0x0010;
pub const FLAG_FLIPPED_X: u16 = 0x0020;
pub const FLAG_IS_ON: u16 = 0x0040;
pub const FLAG_PUBLIC: u16 = 0x0080;
pub const FLAG_GLUED: u16 = 0x0200;
pub const FLAG_ON_FIRE: u16 = 0x0400;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tile {
    pub foreground: u16,
    pub background: u16,
    pub parent: u16,
    pub flags: u16,
    pub tree: Option<Tree>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tree {
    pub planted_at: u64,
    pub grow_seconds: Option<u32>,
    pub fruit: u8,
}

impl Tree {
    pub fn mature(&self, now: u64) -> bool {
        self.grow_seconds.is_some_and(|duration| now.saturating_sub(self.planted_at) >= u64::from(duration))
    }
}

impl Tile {
    #[must_use]
    pub fn new(foreground: u16, background: u16) -> Self {
        Self {
            foreground,
            background,
            parent: 0,
            flags: 0,
            tree: None,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.foreground == 0 && self.background == 0
    }

    #[must_use]
    pub fn is_solid(&self) -> bool {
        self.foreground != 0
    }

    #[must_use]
    pub fn has_extra(&self) -> bool {
        self.flags & FLAG_HAS_EXTRA != 0
    }

    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.flags & FLAG_LOCKED != 0
    }

    #[must_use]
    pub fn is_on(&self) -> bool {
        self.flags & FLAG_IS_ON != 0
    }

    pub fn set_flag(&mut self, flag: u16, enabled: bool) {
        if enabled {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    #[must_use]
    pub fn packed_flags(&self) -> [u8; 2] {
        [(self.flags & 0xff) as u8, (self.flags >> 8) as u8]
    }

    #[must_use]
    pub fn from_packed_flags(low: u8, high: u8) -> u16 {
        u16::from(low) | (u16::from(high) << 8)
    }
}
