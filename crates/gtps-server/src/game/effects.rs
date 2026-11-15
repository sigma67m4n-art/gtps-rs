use super::{tools, Game};
use gtps_enet::PeerId;
use gtps_proto::{framing, TankPacket, Variant, VariantList};

impl Game {
    pub(super) fn send_character_state(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else { return; };
        let mut packet = TankPacket::new(20);
        packet.net_id = session.net_id;
        packet.pad2 = 128;
        packet.pad3 = 128;
        packet.float1 = 125.0;
        packet.int_data = if session.cheats.modfly { 1 << 1 } else { 0 };
        if session.equipped_back == tools::RIPPER_WINGS && session.inventory.count_of(tools::RIPPER_WINGS) > 0 { packet.int_data |= 1; }
        packet.pos_x = 1000.0;
        packet.pos_y = 150.0;
        packet.speed_x = if session.cheats.high_jump { 500.0 } else { 1000.0 };
        packet.speed_y = if session.cheats.super_speed { 500.0 } else { 250.0 };
        packet.tile_x = tools::reach(&session.inventory,session.equipped_hand) as i32;
        packet.tile_y = 1;
        let targets = session.world.as_ref().map_or_else(|| vec![peer], |name| self.world_peers(name,None));
        let hand = if session.inventory.count_of(session.equipped_hand) > 0 { session.equipped_hand } else { 0 };
        let back = if session.inventory.count_of(session.equipped_back) > 0 { session.equipped_back } else { 0 };
        let clothes = VariantList::new("OnSetClothing")
            .arg(Variant::Vec3(0.0,0.0,0.0))
            .arg(Variant::Vec3(0.0,0.0,hand as f32))
            .arg(Variant::Vec3(back as f32,0.0,0.0))
            .arg(Variant::Uint(0x8295c3ff));
        for target in targets {
            self.send_tank(target,&packet,&[]);
            self.send_variant(target,&clothes,packet.net_id,0);
        }
    }

    pub(super) fn award_gems(&mut self, peer: PeerId, amount: u32) {
        if let Some(session) = self.sessions.get_mut(&peer) {
            let amount = amount.min((i32::MAX as u32).saturating_sub(session.gems));
            session.gems += amount;
            session.collected_gems = session.collected_gems.saturating_add(amount);
        }
        self.send_gems(peer);
    }

    pub(super) fn flush_gem_bubble(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get_mut(&peer) else { return; };
        let amount = std::mem::take(&mut session.collected_gems);
        if amount == 0 { return; }
        let net_id = session.net_id;
        let list = VariantList::new("OnTalkBubble").arg(Variant::Int(net_id))
            .arg(Variant::String(format!("Collected {} Gems (gems)", commas(amount))))
            .arg(Variant::Int(0));
        self.send_variant(peer,&list,framing::NET_ID_BROADCAST,0);
    }

    pub(super) fn collect_gem(&mut self, peer: PeerId, uid: u32) -> bool {
        let Some(session) = self.sessions.get(&peer).cloned() else { return false; };
        let Some(name) = session.world else { return false; };
        let Some(world) = self.worlds.get_mut(&name) else { return false; };
        let Some(object) = world.objects.iter().find(|o| o.uid == uid && o.item_id == 112).copied() else { return false; };
        if !object.pos_x.is_finite() || !object.pos_y.is_finite() || (object.pos_x-session.pos_x).abs() > 64.0 || (object.pos_y-session.pos_y).abs() > 64.0 || u32::from(object.count) > (i32::MAX as u32).saturating_sub(session.gems) { return true; }
        world.remove_object(uid);
        self.award_gems(peer,u32::from(object.count));
        let mut packet = TankPacket::new(gtps_proto::tank::TYPE_ITEM_CHANGE_OBJECT);
        packet.net_id = session.net_id;
        packet.int_data = uid as i32;
        for target in self.world_peers(&name,None) { self.send_tank(target,&packet,&[]); }
        true
    }

    pub(super) fn auto_collect_gems(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else { return; };
        if !session.cheats.auto_collect_gems || session.frozen { return; }
        let Some(world) = session.world.as_ref().and_then(|name| self.worlds.get(name)) else { return; };
        let objects: Vec<_> = world.objects.iter().filter(|o| o.item_id == 112 && (o.pos_x-session.pos_x).abs() <= 64.0 && (o.pos_y-session.pos_y).abs() <= 64.0).take(128).map(|o| o.uid).collect();
        for uid in objects { self.collect_gem(peer,uid); }
    }
}

fn commas(amount: u32) -> String {
    let digits = amount.to_string();
    let mut text = String::new();
    for (index,c) in digits.chars().enumerate() {
        if index > 0 && (digits.len()-index)%3 == 0 { text.push(','); }
        text.push(c);
    }
    text
}
