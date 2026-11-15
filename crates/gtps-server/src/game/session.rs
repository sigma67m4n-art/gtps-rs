use gtps_enet::PeerId;
use std::time::Instant;

use crate::game::cheats::CheatState;
use crate::game::inventory::Inventory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Connected,
    LoggedIn,
    InGame,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub peer: PeerId,
    pub stage: Stage,
    pub user_id: u32,
    pub name: String,
    pub country: String,
    pub gems: u32,
    pub world: Option<String>,
    pub net_id: i32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub inventory: Inventory,
    pub cheats: CheatState,
    pub active_magplant: Option<usize>,
    pub remote_hits: u32,
    pub tax_credits: u64,
    pub growtokens: u32,
    pub spray_tokens: u32,
    pub equipped_hand: u16,
    pub last_punch: Option<Instant>,
    pub dialog: Option<(String, u64)>,
    pub dialog_sequence: u64,
}

impl Session {
    #[must_use]
    pub fn new(peer: PeerId) -> Self {
        Self {
            peer,
            stage: Stage::Connected,
            user_id: 0,
            name: String::new(),
            country: "us".to_string(),
            gems: 0,
            world: None,
            net_id: 0,
            pos_x: 0.0,
            pos_y: 0.0,
            inventory: Inventory::default(),
            cheats: CheatState::default(),
            active_magplant: None,
            remote_hits: 0,
            tax_credits: 0,
            growtokens: 0,
            spray_tokens: 0,
            equipped_hand: 0,
            last_punch: None,
            dialog: None,
            dialog_sequence: 0,
        }
    }

    #[must_use]
    pub fn in_world(&self, name: &str) -> bool {
        self.world.as_deref() == Some(name)
    }
}
