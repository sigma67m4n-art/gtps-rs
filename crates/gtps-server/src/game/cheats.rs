pub const OPTIONS: [(&str, &str); 4] = [
    ("auto_farm", "Auto Farm"),
    ("bfg_mode", "Auto Farm: BFG Mode"),
    ("auto_grab_remote", "Auto Grab Magplant Remote"),
    ("use_blocks", "Auto Farm: Use Placed Blocks"),
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheatState {
    pub auto_farm: bool,
    pub bfg_mode: bool,
    pub auto_grab_remote: bool,
    pub use_blocks: bool,
}

impl CheatState {
    pub const BUTTON_PREFIX: &'static str = "cheat_";

    #[must_use]
    pub fn entries(&self) -> [(&'static str, &'static str, bool); 4] {
        OPTIONS.map(|(name, label)| (name, label, self.is_enabled(name)))
    }

    #[must_use]
    pub fn is_enabled(&self, name: &str) -> bool {
        match name {
            "auto_farm" => self.auto_farm,
            "bfg_mode" => self.bfg_mode,
            "auto_grab_remote" => self.auto_grab_remote,
            "use_blocks" => self.use_blocks,
            _ => false,
        }
    }

    pub fn toggle(&mut self, name: &str) -> bool {
        match name {
            "auto_farm" => self.auto_farm = !self.auto_farm,
            "bfg_mode" => self.bfg_mode = !self.bfg_mode,
            "auto_grab_remote" => self.auto_grab_remote = !self.auto_grab_remote,
            "use_blocks" => self.use_blocks = !self.use_blocks,
            _ => return false,
        }

        true
    }

    #[must_use]
    pub fn from_button(button: &str) -> Option<&'static str> {
        let name = button.strip_prefix(Self::BUTTON_PREFIX)?;
        OPTIONS
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(candidate, _)| *candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::CheatState;

    #[test]
    fn toggle_flips_known_flags_only() {
        let mut cheats = CheatState::default();

        assert!(cheats.toggle("auto_farm"));
        assert!(cheats.auto_farm);

        assert!(cheats.toggle("auto_farm"));
        assert!(!cheats.auto_farm);

        assert!(!cheats.toggle("nope"));
    }

    #[test]
    fn entries_report_state() {
        let cheats = CheatState {
            bfg_mode: true,
            ..CheatState::default()
        };

        let bfg = cheats
            .entries()
            .into_iter()
            .find(|(name, _, _)| *name == "bfg_mode")
            .expect("bfg entry");

        assert!(bfg.2);
    }

    #[test]
    fn buttons_map_back_to_options() {
        assert_eq!(CheatState::from_button("cheat_bfg_mode"), Some("bfg_mode"));
        assert_eq!(CheatState::from_button("cheat_nope"), None);
        assert_eq!(CheatState::from_button("bfg_mode"), None);
    }
}
