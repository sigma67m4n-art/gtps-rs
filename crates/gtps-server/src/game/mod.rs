pub mod cheats;
pub mod economy;
pub mod inventory;
pub mod locks;
pub mod login;
pub mod session;
pub mod shop;
pub mod world;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gtps_enet::{Event, Host, Packet, PeerId};
use gtps_proto::tank::{
    TYPE_ITEM_ACTIVATE_OBJECT_REQUEST, TYPE_ITEM_ACTIVATE_REQUEST, TYPE_ITEM_CHANGE_OBJECT,
    TYPE_MODIFY_ITEM_INVENTORY, TYPE_PING_REPLY, TYPE_SEND_INVENTORY_STATE,
    TYPE_SEND_ITEM_DATABASE_DATA, TYPE_SEND_MAP_DATA, TYPE_SEND_TILE_UPDATE_DATA, TYPE_STATE,
    TYPE_TILE_CHANGE_REQUEST,
};
use gtps_proto::{framing, ItemsDat, Message, TankPacket, Text, Variant, VariantList};

use crate::config::Config;
use crate::logging;
use crate::store::WorldStore;

use self::cheats::CheatState;
use session::{Session, Stage};

const SERVICE_TIMEOUT_MS: u32 = 10;
const CHEAT_TICK_MS: u64 = 500;
const CHEAT_ACTIONS_PER_TICK: u32 = 20;

pub struct Game {
    host: Host,
    config: Config,
    worlds: WorldStore,
    sessions: HashMap<PeerId, Session>,
    items: Option<ItemsDat>,
    next_user_id: u32,
    next_net_id: u32,
    rng: u32,
}

fn allocate(counter: &mut u32) -> u32 {
    let value = *counter;
    *counter = counter.wrapping_add(1);
    value
}

fn is_valid_world_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 24 && name.chars().all(|c| c.is_ascii_alphanumeric())
}

fn load_items(config: &Config) -> Option<ItemsDat> {
    match ItemsDat::load(&config.items_path) {
        Ok(items) => {
            logging::info(format!(
                "loaded items.dat v{} with {} items ({} bytes)",
                items.version(),
                items.count(),
                items.len()
            ));
            Some(items)
        }
        Err(error) => {
            logging::warn(format!(
                "no items.dat at {} ({error}); item database requests will be ignored",
                config.items_path
            ));
            None
        }
    }
}

impl Game {
    pub fn new(config: Config) -> Result<Self, gtps_enet::Error> {
        gtps_enet::initialize()?;
        let host = Host::server(config.game_bind, config.max_peers, config.channels)?;
        let items = load_items(&config);

        Ok(Self {
            host,
            config,
            worlds: WorldStore::new(),
            sessions: HashMap::new(),
            items,
            next_user_id: 1,
            next_net_id: 1,
            rng: 0x1234_5678,
        })
    }

    pub fn run(&mut self) -> Result<(), gtps_enet::Error> {
        let mut last_tick = Instant::now();

        loop {
            if let Some(event) = self.host.service(SERVICE_TIMEOUT_MS)? {
                self.handle_event(event);
            }

            if last_tick.elapsed() >= Duration::from_millis(CHEAT_TICK_MS) {
                last_tick = Instant::now();
                self.tick_cheats();
            }
        }
    }

    fn next_rand(&mut self) -> u32 {
        let mut state = self.rng;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.rng = state;
        state
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Connect { peer, .. } => self.on_connect(peer),
            Event::Disconnect { peer, .. } => self.on_disconnect(peer),
            Event::Receive { peer, packet, .. } => {
                let data = packet.data().to_vec();
                self.on_receive(peer, &data);
            }
        }
    }

    fn on_connect(&mut self, peer: PeerId) {
        logging::info(format!("peer {} connected", peer.index()));
        self.sessions.insert(peer, Session::new(peer));
        self.send_raw(peer, framing::server_hello());
    }

    fn on_disconnect(&mut self, peer: PeerId) {
        logging::info(format!("peer {} disconnected", peer.index()));

        if let Some(session) = self.sessions.remove(&peer) {
            self.announce_leave(&session);
        }
    }

    fn on_receive(&mut self, peer: PeerId, data: &[u8]) {
        match Message::decode(data) {
            Ok(Message::GenericText(text)) => self.on_text(peer, &Text::parse(text.as_bytes())),
            Ok(Message::GamePacket { tank, extra }) => self.on_tank(peer, &tank, &extra),
            Ok(_) => {}
            Err(error) => logging::debug(format!(
                "peer {} sent an undecodable packet: {error}",
                peer.index()
            )),
        }
    }

    fn on_text(&mut self, peer: PeerId, text: &Text) {
        match text.get("action") {
            Some(action) => self.on_action(peer, action, text),
            None => self.on_login(peer, text),
        }
    }

    fn on_login(&mut self, peer: PeerId, text: &Text) {
        let Some(request) = login::LoginRequest::parse(text) else {
            return;
        };

        let user_id = allocate(&mut self.next_user_id);
        let name = request.display_name().to_string();
        let country = request.country.clone();
        let gems = self.config.starting_gems;
        let capacity = self.config.inventory_capacity;

        let Some(session) = self.sessions.get_mut(&peer) else {
            return;
        };

        session.user_id = user_id;
        session.name = name.clone();
        session.country = country;
        session.gems = gems;
        session.inventory.set_capacity(capacity);
        session.stage = Stage::LoggedIn;

        let verb = if request.is_transfer() {
            "transferred in"
        } else {
            "logged in"
        };

        logging::info(format!(
            "peer {} {verb} as {name} (user {user_id})",
            peer.index()
        ));

        let hash = self.items.as_ref().map_or(0, ItemsDat::hash);
        let welcome = login::super_main_start(hash, &self.config.cdn_host);
        self.send_variant(peer, &welcome, framing::NET_ID_BROADCAST, 0);

        let motd = login::console_message(&self.config.motd);
        self.send_variant(peer, &motd, framing::NET_ID_BROADCAST, 0);

        self.send_gems(peer);
    }

    fn on_action(&mut self, peer: PeerId, action: &str, text: &Text) {
        match action {
            "enter_game" => self.on_enter_game(peer),
            "join_request" => {
                if let Some(name) = world::join_request(text) {
                    self.enter_world(peer, &name);
                }
            }
            "refresh_item_data" => self.on_refresh_item_data(peer),
            "say" => self.on_say(peer, text),
            "dialog_return" => self.on_dialog_return(peer, text),
            "wrench" => self.open_cheats(peer),
            "quit" | "quit_to_exit" | "leave_world" => self.leave_current_world(peer),
            other => logging::debug(format!("peer {} sent action {other}", peer.index())),
        }
    }

    fn on_say(&mut self, peer: PeerId, text: &Text) {
        let Some(message) = text.get("message") else {
            return;
        };

        match message.trim().to_lowercase().as_str() {
            "/cheats" => self.open_cheats(peer),
            "/shop" => self.open_telephone(peer),
            "/gems" => {
                let gems = self.sessions.get(&peer).map_or(0, |session| session.gems);
                self.console(peer, &format!("You have {gems} gems."));
            }
            _ => {}
        }
    }

    fn on_dialog_return(&mut self, peer: PeerId, text: &Text) {
        match text.get("dialog_name") {
            Some(shop::DIALOG_TELEPHONE) => self.on_telephone_return(peer, text),
            Some(shop::DIALOG_CHEATS) => self.on_cheats_return(peer, text),
            _ => {}
        }
    }

    fn on_telephone_return(&mut self, peer: PeerId, text: &Text) {
        let Some(spec) = shop::lock_button(text) else {
            self.open_telephone(peer);
            return;
        };

        let price = u32::try_from(locks::total_with_tax(spec.shop_price)).unwrap_or(u32::MAX);
        let affordable = self.sessions.get(&peer).is_some_and(|s| s.gems >= price);

        if !affordable {
            self.console(peer, "You do not have enough gems for that lock.");
            return;
        }

        if let Some(session) = self.sessions.get_mut(&peer) {
            session.gems -= price;
            session.inventory.add(spec.item_id, 1);
        }

        self.send_gems(peer);
        self.console(peer, &format!("Bought {} for {price} gems.", spec.name));
    }

    fn on_cheats_return(&mut self, peer: PeerId, text: &Text) {
        if let Some(name) = shop::cheat_button(text) {
            if let Some(session) = self.sessions.get_mut(&peer) {
                session.cheats.toggle(name);
            }
        }

        self.open_cheats(peer);
    }

    fn open_telephone(&mut self, peer: PeerId) {
        let gems = self.sessions.get(&peer).map_or(0, |session| session.gems);
        let dialog = shop::telephone_dialog(gems);
        self.send_variant(peer, &dialog, framing::NET_ID_BROADCAST, 0);
    }

    fn open_cheats(&mut self, peer: PeerId) {
        let cheats = self
            .sessions
            .get(&peer)
            .map_or_else(CheatState::default, |session| session.cheats);
        let dialog = shop::cheats_dialog(&cheats);
        self.send_variant(peer, &dialog, framing::NET_ID_BROADCAST, 0);
    }

    fn send_gems(&mut self, peer: PeerId) {
        let Some(gems) = self.sessions.get(&peer).map(|session| session.gems) else {
            return;
        };

        let list = VariantList::new("OnSetBux").arg(Variant::Int(gems as i32));
        self.send_variant(peer, &list, framing::NET_ID_BROADCAST, 0);
    }

    fn console(&mut self, peer: PeerId, message: &str) {
        let list = login::console_message(message);
        self.send_variant(peer, &list, framing::NET_ID_BROADCAST, 0);
    }

    fn on_enter_game(&mut self, peer: PeerId) {
        if let Some(session) = self.sessions.get_mut(&peer) {
            session.stage = Stage::InGame;
        }

        let menu = login::world_select_menu(&self.config.server_name, &self.config.motd);
        self.send_variant(peer, &menu, framing::NET_ID_BROADCAST, 0);
    }

    fn on_refresh_item_data(&mut self, peer: PeerId) {
        let Some(raw) = self.items.as_ref().map(|items| items.raw().to_vec()) else {
            logging::warn("client asked for items.dat but no database is loaded");
            return;
        };

        let mut tank = TankPacket::new(TYPE_SEND_ITEM_DATABASE_DATA);
        tank.net_id = framing::NET_ID_BROADCAST;
        tank.set_extended(raw.len());
        self.send_tank(peer, &tank, &raw);

        logging::debug(format!(
            "sent items.dat ({} bytes) to peer {}",
            raw.len(),
            peer.index()
        ));
    }

    fn enter_world(&mut self, peer: PeerId, world_name: &str) {
        if !is_valid_world_name(world_name) {
            logging::warn(format!(
                "peer {} asked for an invalid world name",
                peer.index()
            ));
            let list = login::failed_to_enter();
            self.send_variant(peer, &list, framing::NET_ID_BROADCAST, 0);
            return;
        }

        self.leave_current_world(peer);

        let (spawn_x, spawn_y) = world::spawn_position(gtps_world::world::DOOR_ROW);
        let net_id = allocate(&mut self.next_net_id) as i32;

        let Some(session) = self.sessions.get_mut(&peer) else {
            return;
        };

        session.world = Some(world_name.to_string());
        session.net_id = net_id;
        session.pos_x = spawn_x;
        session.pos_y = spawn_y;
        session.stage = Stage::InGame;

        let generated = !self.worlds.is_loaded(world_name);
        self.worlds.ensure(world_name);

        if generated {
            logging::info(format!("generated world {world_name}"));
        }

        let map = gtps_world::codec::encode(
            self.worlds
                .get(world_name)
                .expect("world was just ensured to exist"),
        );

        let mut map_tank = TankPacket::new(TYPE_SEND_MAP_DATA);
        map_tank.net_id = framing::NET_ID_BROADCAST;
        map_tank.set_extended(map.len());
        self.send_tank(peer, &map_tank, &map);

        let others: Vec<Session> = self
            .sessions
            .values()
            .filter(|session| session.in_world(world_name) && session.peer != peer)
            .cloned()
            .collect();
        let me = self.sessions.get(&peer).cloned();

        for other in &others {
            self.send_spawn(peer, world::spawn_string(other, false));
        }

        if let Some(me) = &me {
            logging::debug(format!(
                "peer {} carries {}/{} inventory slots",
                peer.index(),
                me.inventory.items().len(),
                me.inventory.capacity()
            ));

            self.send_spawn(peer, world::spawn_string(me, true));

            let inventory = me.inventory.encode();
            let mut tank = TankPacket::new(TYPE_SEND_INVENTORY_STATE);
            tank.net_id = me.net_id;
            tank.set_extended(inventory.len());
            self.send_tank(peer, &tank, &inventory);

            let broadcast = world::spawn_string(me, false);
            let targets = self.world_peers(world_name, Some(peer));
            for target in targets {
                self.send_spawn(target, broadcast.clone());
            }
        }

        logging::info(format!("peer {} entered world {world_name}", peer.index()));
    }

    fn on_tank(&mut self, peer: PeerId, tank: &TankPacket, _extra: &[u8]) {
        match tank.kind {
            TYPE_STATE => self.broadcast_state(peer, tank),
            TYPE_TILE_CHANGE_REQUEST => self.on_tile_change(peer, tank),
            TYPE_ITEM_ACTIVATE_REQUEST => self.on_item_activate(peer, tank),
            TYPE_ITEM_ACTIVATE_OBJECT_REQUEST => self.on_collect(peer, tank),
            TYPE_PING_REPLY => logging::debug(format!("peer {} replied to a ping", peer.index())),
            other => logging::debug(format!("peer {} sent tank type {other}", peer.index())),
        }
    }

    fn on_item_activate(&mut self, peer: PeerId, tank: &TankPacket) {
        let item = u16::try_from(tank.int_data).unwrap_or(0);

        if item == locks::TELEPHONE_ITEM {
            self.open_telephone(peer);
        }
    }

    fn broadcast_state(&mut self, peer: PeerId, tank: &TankPacket) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        let Some(world_name) = session.world.clone() else {
            return;
        };

        if let Some(stored) = self.sessions.get_mut(&peer) {
            stored.pos_x = tank.pos_x;
            stored.pos_y = tank.pos_y;
        }

        let mut out = *tank;
        out.net_id = session.net_id;

        for target in self.world_peers(&world_name, Some(peer)) {
            self.send_tank(target, &out, &[]);
        }
    }

    fn on_tile_change(&mut self, peer: PeerId, tank: &TankPacket) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        let Some(world_name) = session.world.clone() else {
            return;
        };
        let (Ok(x), Ok(y)) = (usize::try_from(tank.tile_x), usize::try_from(tank.tile_y)) else {
            return;
        };

        let item = u16::try_from(tank.int_data).unwrap_or(0);

        if item != 0 {
            let available = self
                .sessions
                .get(&peer)
                .is_some_and(|stored| stored.inventory.count_of(item) > 0);

            if !available {
                return;
            }

            let taken = self
                .sessions
                .get_mut(&peer)
                .is_some_and(|stored| stored.inventory.remove(item, 1));

            if !taken {
                return;
            }

            let mut modify = TankPacket::new(TYPE_MODIFY_ITEM_INVENTORY);
            modify.net_id = session.net_id;
            modify.int_data = i32::from(item);
            modify.secondary_id = -1;
            self.send_tank(peer, &modify, &[]);
        }

        let tile = if item == 0 {
            gtps_world::Tile::default()
        } else {
            gtps_world::Tile::new(item, 0)
        };

        let Some(world) = self.worlds.get_mut(&world_name) else {
            return;
        };
        world.set_tile(x, y, tile);
        let Some(updated) = world.tile(x, y).copied() else {
            return;
        };

        let payload = gtps_world::codec::encode_tile(&updated);

        if let Some(index) = gtps_world::World::index(x, y) {
            match locks::by_item(item) {
                Some(spec) => {
                    self.worlds
                        .ensure_magplant(&world_name, index, spec.item_id);
                }
                None => self.worlds.remove_magplant(&world_name, index),
            }
        }

        let mut update = TankPacket::new(TYPE_SEND_TILE_UPDATE_DATA);
        update.net_id = session.net_id;
        update.tile_x = tank.tile_x;
        update.tile_y = tank.tile_y;
        update.set_extended(payload.len());

        for target in self.world_peers(&world_name, None) {
            self.send_tank(target, &update, &payload);
        }
    }

    fn on_collect(&mut self, peer: PeerId, tank: &TankPacket) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        let Some(world_name) = session.world.clone() else {
            return;
        };

        let uid = tank.int_data as u32;
        let collected = self
            .worlds
            .get_mut(&world_name)
            .and_then(|world| world.remove_object(uid));

        let Some(object) = collected else {
            return;
        };

        if let Some(stored) = self.sessions.get_mut(&peer) {
            stored
                .inventory
                .add(object.item_id, object.count.min(u16::from(u8::MAX)) as u8);
        }

        let mut out = TankPacket::new(TYPE_ITEM_CHANGE_OBJECT);
        out.net_id = session.net_id;
        out.int_data = uid as i32;

        for target in self.world_peers(&world_name, None) {
            self.send_tank(target, &out, &[]);
        }
    }

    fn tick_cheats(&mut self) {
        let peers: Vec<PeerId> = self
            .sessions
            .values()
            .filter(|session| session.cheats.auto_farm && session.world.is_some())
            .map(|session| session.peer)
            .collect();

        for peer in peers {
            self.run_auto_farm(peer);
        }
    }

    fn run_auto_farm(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        let Some(world_name) = session.world.clone() else {
            return;
        };

        let roll = self.next_rand();
        let mut reward = None;

        if session.cheats.bfg_mode {
            let reach = self.farm_reach(&session);
            let taken = self.worlds.take_from_magplant(
                &world_name,
                session.active_magplant,
                session.cheats.auto_grab_remote,
                CHEAT_ACTIONS_PER_TICK,
                reach,
            );

            if let Some((tile, lock_item, count)) = taken {
                let multiplier = locks::multiplier_percent(lock_item);
                let per_cycle = economy::cycle_gems(economy::highest(), multiplier, roll);

                if let Some(stored) = self.sessions.get_mut(&peer) {
                    stored.active_magplant = Some(tile);
                }

                reward = Some(per_cycle * count);
            }
        }

        if reward.is_none() && (session.cheats.use_blocks || session.cheats.bfg_mode) {
            reward = self.consume_block(peer, roll);
        }

        if let Some(gems) = reward.filter(|gems| *gems > 0) {
            if let Some(stored) = self.sessions.get_mut(&peer) {
                stored.gems = stored.gems.saturating_add(gems);
            }

            self.send_gems(peer);
        }
    }

    fn consume_block(&mut self, peer: PeerId, roll: u32) -> Option<u32> {
        let session = self.sessions.get(&peer)?;
        let farmable = economy::table()
            .iter()
            .filter(|farmable| session.inventory.count_of(farmable.block_id) > 0)
            .max_by_key(|farmable| farmable.break_gems)?;
        let block_id = farmable.block_id;
        let gems = economy::cycle_gems(farmable, economy::GEMS_MULTIPLIER_BASE, roll);

        let stored = self.sessions.get_mut(&peer)?;
        stored.inventory.remove(block_id, 1);

        Some(gems)
    }

    fn farm_reach(&self, session: &Session) -> Option<(usize, usize, usize)> {
        session.world.as_ref()?;

        let range = if session.inventory.count_of(locks::RAYMAN_FIST) > 0 {
            locks::RAYMAN_FIST_RANGE
        } else {
            locks::FIST_RANGE
        };

        let x = TankPacket::pixel_to_tile(session.pos_x).max(0) as usize;
        let y = TankPacket::pixel_to_tile(session.pos_y).max(0) as usize;

        Some((x, y, range))
    }

    fn leave_current_world(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };

        if session.world.is_none() {
            return;
        }

        self.announce_leave(&session);

        if let Some(stored) = self.sessions.get_mut(&peer) {
            stored.world = None;
            stored.active_magplant = None;
        }
    }

    fn announce_leave(&mut self, session: &Session) {
        let Some(world_name) = session.world.clone() else {
            return;
        };

        let payload = world::remove_string(session.net_id, session.user_id);
        for target in self.world_peers(&world_name, Some(session.peer)) {
            let list = VariantList::new("OnRemove").arg(Variant::String(payload.clone()));
            self.send_variant(target, &list, framing::NET_ID_BROADCAST, 0);
        }
    }

    fn world_peers(&self, world_name: &str, exclude: Option<PeerId>) -> Vec<PeerId> {
        self.sessions
            .values()
            .filter(|session| session.in_world(world_name) && Some(session.peer) != exclude)
            .map(|session| session.peer)
            .collect()
    }

    fn send_spawn(&mut self, peer: PeerId, payload: String) {
        let list = VariantList::new("OnSpawn").arg(Variant::String(payload));
        self.send_variant(peer, &list, framing::NET_ID_BROADCAST, 0);
    }

    fn send_raw(&mut self, peer: PeerId, data: Vec<u8>) {
        if let Err(error) = self.host.send(peer, 0, Packet::reliable(data)) {
            logging::warn(format!("send to peer {} failed: {error}", peer.index()));
        }
    }

    fn send_variant(&mut self, peer: PeerId, list: &VariantList, net_id: i32, delay: i32) {
        let frame = framing::variant_packet(list, net_id, delay);
        self.send_raw(peer, frame);
    }

    fn send_tank(&mut self, peer: PeerId, tank: &TankPacket, extra: &[u8]) {
        let frame = framing::game_packet(tank, extra);
        self.send_raw(peer, frame);
    }
}
