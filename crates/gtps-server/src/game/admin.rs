use super::{acquisition, blast, locks, tools, Game};
use gtps_enet::PeerId;
use gtps_proto::{framing, Text, Variant, VariantList};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Player,
    Moderator,
    Owner,
}

impl Role {
    pub fn from_value(value: u8) -> Self {
        match value {
            2 => Self::Owner,
            1 => Self::Moderator,
            _ => Self::Player,
        }
    }
}

fn dimensions(value: &str) -> Option<(usize, usize)> {
    let (x, y) = value.split_once('x')?;
    let (x, y) = (x.parse().ok()?, y.parse().ok()?);
    ((10..=200).contains(&x) && (10..=200).contains(&y)).then_some((x, y))
}

impl Game {
    pub(super) fn admin_command(&mut self, peer: PeerId, message: &str) -> bool {
        let mut args = message.split_whitespace();
        let command = args.next().unwrap_or("").to_ascii_lowercase();
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return false;
        };
        if command == "/owner" {
            if session
                .auth_attempt
                .is_some_and(|time| time.elapsed().as_secs() < 2)
            {
                return true;
            }
            if let Some(stored) = self.sessions.get_mut(&peer) {
                stored.auth_attempt = Some(std::time::Instant::now());
            }
            let key = args.next().unwrap_or("");
            if !self.config.owner_key.is_empty()
                && key == self.config.owner_key
                && self.auth.set_role(session.user_id, 2).is_ok()
            {
                if let Some(stored) = self.sessions.get_mut(&peer) {
                    stored.role = Role::Owner;
                }
                self.console(peer, "Owner role attached to your account.");
            } else {
                self.console(peer, "Owner authentication failed.");
            }
            return true;
        }
        let moderator = matches!(
            command.as_str(),
            "/kick"
                | "/ban"
                | "/unban"
                | "/mute"
                | "/unmute"
                | "/freeze"
                | "/unfreeze"
                | "/summon"
                | "/goto"
                | "/who"
                | "/help"
        );
        let owner = matches!(
            command.as_str(),
            "/give"
                | "/drop"
                | "/clearworld"
                | "/cleardrop"
                | "/createworld"
                | "/setrole"
                | "/setgems"
        );
        if !moderator && !owner {
            return false;
        }
        if session.role < Role::Moderator || (owner && session.role != Role::Owner) {
            self.console(peer, "This command requires a higher role.");
            return true;
        }
        match command.as_str() {
            "/help" => self.console(peer, "/who /goto NAME /summon NAME /kick NAME /ban NAME /unban ACCOUNT_ID /mute NAME /unmute NAME /freeze NAME /unfreeze NAME. Owner: /give ID COUNT /drop ID COUNT /setgems AMOUNT /setrole NAME player|moderator /createworld WIDTHxHEIGHT /clearworld /cleardrop"),
            "/who" => {
                let names = self.sessions.values().filter(|s| s.user_id != 0).map(|s| format!("{} ({}, {:?})", s.name, s.user_id, s.role)).collect::<Vec<_>>().join(", ");
                self.console(peer, &names);
            }
            "/unban" => {
                if let Some(id) = args.next().and_then(|s| s.parse().ok()) {
                    if self.auth.ban(id, false).is_ok() { self.console(peer, "Account unbanned."); }
                }
            }
            "/setgems" => {
                if let Some(amount) = args.next().and_then(|s| s.parse::<u32>().ok()).filter(|n| *n <= i32::MAX as u32) {
                    if let Some(stored) = self.sessions.get_mut(&peer) { stored.gems = amount; }
                    self.send_gems(peer);
                }
            }
            "/give" | "/drop" => {
                let item = args.next().and_then(|s| s.parse::<u16>().ok());
                let amount = args.next().and_then(|s| s.parse::<u8>().ok()).filter(|n| (1..=super::inventory::MAX_STACK).contains(n));
                if let (Some(item), Some(amount)) = (item, amount) {
                    let known = self.items.as_ref().is_some_and(|db| u32::from(item) < db.count()) || blast::by_item(item).is_some() || locks::by_item(item).is_some();
                    if !known || item == 0 || item == 112 || !acquisition::allows_world_reward(item) {
                        self.console(peer, "That item must use its normal acquisition route.");
                    } else if command == "/give" {
                        if self.sessions.get_mut(&peer).is_some_and(|s| s.inventory.add(item,amount)) { self.send_inventory(peer); }
                        else { self.console(peer, "Inventory is full."); }
                    } else if !matches!(item, gtps_world::items::FIST | gtps_world::items::WRENCH) {
                        if let Some(name) = session.world.as_ref() {
                            let room = self.worlds.get(name).is_some_and(|w| w.objects.len() < 10000 && w.next_object_uid < i32::MAX as u32);
                            if room && self.sessions.get_mut(&peer).is_some_and(|s| s.inventory.remove(item,amount)) {
                                if let Some(world) = self.worlds.get_mut(name) { world.add_object(item,session.pos_x,session.pos_y,u16::from(amount)); }
                                self.refresh_world(name);
                                self.send_inventory(peer);
                            }
                        }
                    }
                } else { self.console(peer, "Usage: /give ID COUNT or /drop ID COUNT (1-250)."); }
            }
            "/createworld" => {
                if let Some((width,height)) = args.next().and_then(dimensions) {
                    let name = loop {
                        let name = format!("OWNER{}", super::allocate(&mut self.next_blast));
                        if !self.worlds.is_loaded(&name) { break name; }
                    };
                    self.worlds.ensure_sized(&name,width,height).owner_user_id = session.user_id;
                    self.enter_world(peer,&name);
                    self.console(peer, &format!("Created {name}: {width}x{height}."));
                } else { self.console(peer, "Usage: /createworld 50x50 (each side 10-200)."); }
            }
            "/clearworld" | "/cleardrop" => {
                if let Some(name) = session.world.as_ref() {
                    let entry = self.worlds.entry_mut(name);
                    entry.world.objects.clear();
                    if command == "/clearworld" {
                        let (width,height) = (entry.world.width,entry.world.height);
                        entry.world = gtps_world::World::sized(name,width,height);
                        entry.world.owner_user_id = session.user_id;
                        entry.magplants.clear();
                        entry.damage.clear();
                    }
                    self.refresh_world(name);
                }
            }
            _ => {
                let name = args.next().unwrap_or("");
                let target = self.sessions.values().find(|s| s.name.eq_ignore_ascii_case(name) && s.user_id != 0).cloned();
                if let Some(target) = target {
                    if target.role >= session.role && target.peer != peer {
                        self.console(peer, "You cannot moderate an equal or higher role.");
                        return true;
                    }
                    match command.as_str() {
                        "/kick" | "/ban" => {
                            if command == "/ban" && self.auth.ban(target.user_id,true).is_err() { return true; }
                            self.console(target.peer, "Disconnected by a moderator.");
                            self.on_disconnect(target.peer);
                            self.host.disconnect(target.peer, 0);
                        }
                        "/mute" | "/unmute" => {
                            if let Some(stored) = self.sessions.get_mut(&target.peer) { stored.muted = command == "/mute"; }
                        }
                        "/freeze" | "/unfreeze" => {
                            if let Some(stored) = self.sessions.get_mut(&target.peer) { stored.frozen = command == "/freeze"; }
                            self.send_variant(target.peer,&VariantList::new("OnSetFreezeState").arg(Variant::Int(i32::from(command == "/freeze"))),framing::NET_ID_BROADCAST,0);
                        }
                        "/goto" => { if let Some(name) = target.world { self.enter_world(peer,&name); } }
                        "/summon" => { if let Some(name) = session.world { self.enter_world(target.peer,&name); } }
                        "/setrole" => {
                            let role = match args.next() { Some("player") => 0, Some("moderator") => 1, _ => { self.console(peer,"Use player or moderator."); return true; } };
                            if self.auth.set_role(target.user_id,role).is_ok() {
                                if let Some(stored) = self.sessions.get_mut(&target.peer) { stored.role = Role::from_value(role); }
                            }
                        }
                        _ => {}
                    }
                } else { self.console(peer,"Player not online."); }
            }
        }
        true
    }

    pub(super) fn refresh_world(&mut self, name: &str) {
        let peers = self.world_peers(name, None);
        for peer in peers {
            self.enter_world(peer, name);
        }
    }

    pub(super) fn open_magplant(&mut self, peer: PeerId, index: usize) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        let Some(name) = session.world else {
            return;
        };
        let plant = self
            .worlds
            .entry_mut(&name)
            .magplants
            .iter()
            .find(|p| p.tile == index)
            .copied();
        let Some(plant) = plant else {
            return;
        };
        let maximum = locks::magplant_stock(plant.lock_item);
        let mut body = format!("add_label_with_icon|big|MAGPLANT 5000|left|{}|\nadd_textbox|Stock: {} / {maximum}|left|\nadd_button|get_remote|Get Remote|\n",tools::MAGPLANT,plant.stock);
        if session.role == Role::Owner {
            body.push_str(&format!("add_button|refill|Add {maximum} Stock|\n"));
        }
        body.push_str("end_dialog|magplant|Close||\n");
        if let Some(stored) = self.sessions.get_mut(&peer) {
            stored.magplant_dialog = Some((name, index));
        }
        self.send_variant(
            peer,
            &VariantList::new("OnDialogRequest").arg(Variant::String(body)),
            framing::NET_ID_BROADCAST,
            0,
        );
    }

    pub(super) fn magplant_return(&mut self, peer: PeerId, text: &Text) {
        let Some(session) = self.sessions.get_mut(&peer) else {
            return;
        };
        let Some((name, index)) = session.magplant_dialog.take() else {
            return;
        };
        if session.world.as_ref() != Some(&name) {
            return;
        }
        let session = session.clone();
        let role = session.role;
        let Some((x, y, range)) = self.farm_reach(&session) else {
            return;
        };
        let entry = self.worlds.entry_mut(&name);
        let Some((tx, ty)) = entry.world.coords(index) else {
            return;
        };
        if x.abs_diff(tx) > range
            || y.abs_diff(ty) > range
            || entry.world.tiles[index].foreground != tools::MAGPLANT
        {
            return;
        }
        let Some(plant) = entry.magplants.iter_mut().find(|p| p.tile == index) else {
            return;
        };
        if super::shop::clicked(text, "refill") && role == Role::Owner {
            plant.stock = locks::magplant_stock(plant.lock_item);
        }
        if super::shop::clicked(text, "get_remote") {
            if let Some(stored) = self.sessions.get_mut(&peer) {
                stored.active_magplant = Some(index);
                stored.remote_hits = 0;
            }
        }
        self.open_magplant(peer, index);
    }
}

#[cfg(test)]
mod tests {
    use super::dimensions;
    #[test]
    fn world_dimensions_reject_overflow_and_out_of_bounds() {
        assert_eq!(dimensions("50x200"), Some((50, 200)));
        for value in [
            "0x50",
            "201x50",
            "50x-1",
            "999999999999999999999x50",
            "50x50x50",
        ] {
            assert_eq!(dimensions(value), None);
        }
    }
}
