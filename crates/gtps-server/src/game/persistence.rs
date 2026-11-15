use super::{inventory::Inventory, session::Session, Game};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlayerState {
    gems: u32,
    inventory: Inventory,
    tax_credits: u64,
    growtokens: u32,
    spray_tokens: u32,
    equipped_hand: u16,
    #[serde(default)]
    equipped_back: u16,
}

impl PlayerState {
    pub fn capture(session: &Session) -> Self {
        Self {
            gems: session.gems,
            inventory: session.inventory.clone(),
            tax_credits: session.tax_credits,
            growtokens: session.growtokens,
            spray_tokens: session.spray_tokens,
            equipped_hand: session.equipped_hand,
            equipped_back: session.equipped_back,
        }
    }

    pub fn apply(self, session: &mut Session) {
        session.gems = self.gems;
        session.inventory = self.inventory;
        session.tax_credits = self.tax_credits;
        session.growtokens = self.growtokens;
        session.spray_tokens = self.spray_tokens;
        session.equipped_hand = self.equipped_hand;
        session.equipped_back = self.equipped_back;
    }
}

impl Game {
    pub(super) fn checkpoint(&self) -> gtps_auth::Result<()> {
        let players = self
            .sessions
            .values()
            .filter(|s| s.user_id != 0)
            .map(|s| Ok((s.user_id, serde_json::to_vec(&PlayerState::capture(s))?)))
            .collect::<gtps_auth::Result<Vec<_>>>()?;
        self.auth
            .checkpoint(&players, &serde_json::to_vec(&self.worlds)?)
    }
}
