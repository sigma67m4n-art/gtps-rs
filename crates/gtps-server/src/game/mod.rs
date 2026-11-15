pub mod acquisition;
mod admin;
pub mod blast;
pub mod cheats;
pub mod economy;
pub mod farming;
pub mod inventory;
pub mod locks;
pub mod login;
mod persistence;
mod effects;
pub mod session;
pub mod shop;
pub mod tools;
pub mod world;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gtps_enet::{Event, Host, Packet, PeerId};
use gtps_proto::tank::{
    TYPE_ITEM_ACTIVATE_OBJECT_REQUEST, TYPE_ITEM_ACTIVATE_REQUEST, TYPE_ITEM_CHANGE_OBJECT,
    TYPE_PING_REPLY, TYPE_SEND_INVENTORY_STATE, TYPE_SEND_ITEM_DATABASE_DATA, TYPE_SEND_MAP_DATA,
    TYPE_SEND_TILE_UPDATE_DATA, TYPE_STATE, TYPE_TILE_CHANGE_REQUEST,
};
use gtps_proto::{framing, ItemsDat, Message, TankPacket, Text, Variant, VariantList};

use crate::config::Config;
use crate::logging;
use crate::store::WorldStore;

use self::cheats::CheatState;
use session::{Session, Stage};

const SERVICE_TIMEOUT_MS: u32 = 10;
const CHEAT_TICK_MS: u64 = 500;

pub struct Game {
    host: Host,
    config: Config,
    worlds: WorldStore,
    sessions: HashMap<PeerId, Session>,
    items: Option<ItemsDat>,
    auth: gtps_auth::AuthStore,
    next_net_id: u32,
    next_blast: u32,
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
        Ok(mut items) => {
            items.set_stack_limit(inventory::MAX_STACK);
            items.configure_farmable(economy::highest().block_id, economy::highest().hits as u8);
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
    pub fn new(config: Config) -> gtps_auth::Result<Self> {
        gtps_enet::initialize()?;
        let host = Host::server(config.game_bind, config.max_peers, config.channels)?;
        let items = load_items(&config);
        let auth = gtps_auth::AuthStore::open(&config.auth_path)?;
        let worlds = match auth.world_state()? {
            Some(data) => serde_json::from_slice(&data)?,
            None => WorldStore::new(),
        };

        Ok(Self {
            host,
            config,
            worlds,
            sessions: HashMap::new(),
            items,
            auth,
            next_net_id: 1,
            next_blast: 1,
            rng: 0x1234_5678,
        })
    }

    pub fn run(&mut self) -> gtps_auth::Result<()> {
        let mut last_tick = Instant::now();

        loop {
            if let Some(event) = self.host.service(SERVICE_TIMEOUT_MS)? {
                self.handle_event(event);
                self.checkpoint()?;
            }

            if last_tick.elapsed() >= Duration::from_millis(CHEAT_TICK_MS) {
                last_tick = Instant::now();
                self.tick_cheats();
                self.checkpoint()?;
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
        if let Err(error) = self.checkpoint() {
            logging::error(format!("checkpoint failed: {error}"));
            std::process::exit(1);
        }

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
        if !self
            .sessions
            .get(&peer)
            .is_some_and(|session| session.stage == Stage::Connected)
        {
            return;
        }
        let Some(session) = self.sessions.get_mut(&peer) else { return; };
        if session.auth_attempt.is_some_and(|time| time.elapsed() < Duration::from_secs(2)) { return; }
        session.auth_attempt = Some(Instant::now());
        let account = if let Some(encoded) = text.get("ltoken") {
            gtps_auth::decode_login_token(encoded).and_then(|token| self.auth.verify_token(&token, true))
        } else {
            self.auth.login(text.get("tankIDName").unwrap_or(""), text.get("tankIDPass").unwrap_or(""))
        };
        let Ok(account) = account else {
            self.console(peer, "Login failed. Register or sign in through the GrowID page.");
            return;
        };
        if self.auth.banned(account.id).unwrap_or(true) || self.sessions.values().any(|s| s.user_id == account.id) {
            self.console(peer, "Account banned or already connected.");
            return;
        }
        let state = match self.auth.load_state(account.id) {
            Ok(Some(bytes)) => match serde_json::from_slice::<persistence::PlayerState>(&bytes) {
                Ok(state) => Some(state),
                Err(_) => { self.console(peer, "Account data could not be loaded."); return; }
            },
            Ok(None) => None,
            Err(_) => { self.console(peer, "Account database unavailable."); return; }
        };
        let user_id = account.id;
        let name = account.name;
        let country = text.get("country").filter(|value| value.len() == 2 && value.bytes().all(|c| c.is_ascii_alphabetic())).unwrap_or("us").to_string();
        let gems = self.config.starting_gems.min(i32::MAX as u32);
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
        session.role = admin::Role::from_value(account.role);
        if let Some(state) = state { state.apply(session); }
        else { session.inventory = inventory::Inventory::starter(); session.inventory.set_capacity(capacity.max(5)); }

        logging::info(format!(
            "peer {} logged in as {name} (user {user_id})",
            peer.index()
        ));

        let hash = self.items.as_ref().map_or(0, ItemsDat::hash);
        let welcome = login::super_main_start(hash, &self.config.cdn_host);
        self.send_variant(peer, &welcome, framing::NET_ID_BROADCAST, 0);

        let motd = login::console_message(&self.config.motd);
        self.send_variant(peer, &motd, framing::NET_ID_BROADCAST, 0);

        self.send_gems(peer);
        self.send_tokens(peer);
    }

    fn on_action(&mut self, peer: PeerId, action: &str, text: &Text) {
        if !self
            .sessions
            .get(&peer)
            .is_some_and(|session| session.stage != Stage::Connected)
        {
            return;
        }
        match action {
            "store" => self.open_equipment_shop(peer),
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
        if message.len() > 256 { return; }
        if self.admin_command(peer, message) { return; }
        if self.sessions.get(&peer).is_some_and(|s| s.muted) { return; }

        match message.trim().to_lowercase().as_str() {
            "/cheats" => self.open_cheats(peer),
            "/gems" => {
                let gems = self.sessions.get(&peer).map_or(0, |session| session.gems);
                self.console(peer, &format!("You have {gems} gems."));
            }
            _ => {}
        }
    }

    fn on_dialog_return(&mut self, peer: PeerId, text: &Text) {
        let Some(session) = self.sessions.get_mut(&peer) else {
            return;
        };
        let valid = session.dialog.as_ref().is_some_and(|(name, nonce)| {
            text.get("dialog_name") == Some(name.as_str())
                && text
                    .get("nonce")
                    .and_then(|value| value.parse::<u64>().ok())
                    == Some(*nonce)
        });
        if !valid {
            return;
        }
        session.dialog = None;
        if shop::clicked(text, "close") {
            return;
        }
        match text.get("dialog_name") {
            Some(tools::DIALOG) => self.on_equipment_return(peer, text),
            Some(shop::DIALOG_TELEPHONE_DIAL) => self.on_dial_return(peer, text),
            Some(shop::DIALOG_TELEPHONE) => self.on_telephone_return(peer, text),
            Some(shop::DIALOG_TAX) => self.on_tax_return(peer, text),
            Some(shop::DIALOG_SPRAY) => self.on_spray_return(peer, text),
            Some(shop::DIALOG_CHEATS) => self.on_cheats_return(peer, text),
            Some("magplant") => self.magplant_return(peer, text),
            _ => {}
        }
    }

    fn on_dial_return(&mut self, peer: PeerId, text: &Text) {
        let number = text.get("number").unwrap_or("");

        match shop::service_for(number) {
            Some(shop::Service::Salesman) => self.open_lock_shop(peer),
            Some(shop::Service::Tax) => self.open_tax(peer),
            Some(shop::Service::Spray) => self.open_spray(peer),
            None if !number.trim().is_empty() => {
                self.console(peer, "The number you dialed is not in service.");
            }
            None => {}
        }
    }

    fn on_telephone_return(&mut self, peer: PeerId, text: &Text) {
        if shop::wants_conversion(text) {
            self.convert_blue_gem(peer);
            self.open_lock_shop(peer);
            return;
        }

        if shop::wants_white_conversion(text) {
            self.convert_white_gem(peer);
            self.open_lock_shop(peer);
            return;
        }

        if let Some(service) = shop::open_button(text) {
            match service {
                shop::Service::Tax => self.open_tax(peer),
                shop::Service::Spray => self.open_spray(peer),
                shop::Service::Salesman => self.open_lock_shop(peer),
            }
            return;
        }

        if let Some(spec) = shop::lock_button(text) {
            self.buy_lock(peer, spec);
            self.open_lock_shop(peer);
        }
    }

    fn on_tax_return(&mut self, peer: PeerId, text: &Text) {
        if shop::wants_exchange(text) {
            self.exchange_tokens(peer);
        } else if shop::wants_spray_exchange(text) {
            self.exchange_spray_tokens(peer);
        }

        self.open_tax(peer);
    }

    fn on_spray_return(&mut self, peer: PeerId, text: &Text) {
        if shop::wants_buy_spray(text) {
            self.buy_ultra_spray(peer);
        }

        self.open_spray(peer);
    }

    fn buy_lock(&mut self, peer: PeerId, spec: &locks::LockSpec) {
        if spec.shop_price == 0 {
            return;
        }
        let price = u32::try_from(locks::total_with_tax(spec.shop_price)).unwrap_or(u32::MAX);
        let tax = u32::try_from(locks::tax(spec.shop_price)).unwrap_or(0);

        let can_buy = self.sessions.get(&peer).is_some_and(|session| {
            session.gems >= price && session.inventory.can_add(spec.item_id)
        });

        if !can_buy {
            self.console(peer, "You cannot buy that lock right now.");
            return;
        }

        if let Some(session) = self.sessions.get_mut(&peer) {
            if session.tax_credits.checked_add(u64::from(tax)).is_none()
                || !session.inventory.add(spec.item_id, 1)
            {
                return;
            }
            session.gems -= price;
            session.tax_credits += u64::from(tax);
        }

        self.send_inventory(peer);
        self.send_gems(peer);
        self.console(peer, &format!("Bought {} for {price} gems.", spec.name));
    }

    fn exchange_tokens(&mut self, peer: PeerId) {
        let credits = self
            .sessions
            .get(&peer)
            .map_or(0, |session| session.tax_credits);
        let (tokens, remaining) = economy::exchange_tokens(credits);

        if tokens == 0 {
            self.console(peer, "You need more tax credit to exchange.");
            return;
        }

        if let Some(session) = self.sessions.get_mut(&peer) {
            let Some(balance) = session
                .growtokens
                .checked_add(tokens)
                .filter(|value| *value <= i32::MAX as u32)
            else {
                return;
            };
            session.tax_credits = remaining;
            session.growtokens = balance;
        }

        self.send_tokens(peer);
        self.console(peer, &format!("Exchanged {tokens} Growtokens."));
    }

    fn exchange_spray_tokens(&mut self, peer: PeerId) {
        let credits = self
            .sessions
            .get(&peer)
            .map_or(0, |session| session.tax_credits);
        let (tokens, remaining) = economy::exchange_spray_tokens(credits);

        if tokens == 0 {
            self.console(peer, "You need more tax credit to exchange.");
            return;
        }

        if let Some(session) = self.sessions.get_mut(&peer) {
            let Some(balance) = session.spray_tokens.checked_add(tokens) else {
                return;
            };
            session.tax_credits = remaining;
            session.spray_tokens = balance;
        }

        self.console(peer, &format!("Exchanged {tokens} Tourney Tokens."));
    }

    fn buy_ultra_spray(&mut self, peer: PeerId) {
        let price = economy::ULTRA_WORLD_SPRAY_PRICE;
        let ready = self.sessions.get(&peer).is_some_and(|session| {
            session.spray_tokens >= price && session.inventory.can_add(economy::ULTRA_WORLD_SPRAY)
        });

        if !ready {
            self.console(peer, "You need more Tourney Tokens or inventory space.");
            return;
        }

        if let Some(session) = self.sessions.get_mut(&peer) {
            if !session.inventory.add(economy::ULTRA_WORLD_SPRAY, 1) {
                return;
            }
            session.spray_tokens -= price;
        }

        self.send_inventory(peer);
        self.console(
            peer,
            &format!("Bought an Ultra World Spray for {price} Tourney Tokens."),
        );
    }

    fn convert_blue_gem(&mut self, peer: PeerId) {
        let diamond = locks::by_kind(locks::LockKind::Diamond).item_id;
        let blue_gem = locks::by_kind(locks::LockKind::BlueGem).item_id;
        let needed = locks::DIAMOND_PER_BLUE_GEM;

        let ready = self.sessions.get(&peer).is_some_and(|session| {
            session.inventory.count_of(diamond) >= needed && session.inventory.can_add(blue_gem)
        });

        if !ready {
            self.console(
                peer,
                &format!("You need {needed} Diamond Locks to convert."),
            );
            return;
        }

        let converted = self
            .sessions
            .get_mut(&peer)
            .is_some_and(|session| session.inventory.exchange(diamond, needed as u8, blue_gem));

        if converted {
            self.send_inventory(peer);
            self.console(
                peer,
                &format!("Converted {needed} Diamond Locks into a Blue Gem Lock."),
            );
        }
    }

    fn convert_white_gem(&mut self, peer: PeerId) {
        let blue_gem = locks::by_kind(locks::LockKind::BlueGem).item_id;
        let white_gem = locks::by_kind(locks::LockKind::WhiteGem).item_id;
        let needed = locks::BLUE_GEM_PER_WHITE_GEM;

        let ready = self.sessions.get(&peer).is_some_and(|session| {
            session.inventory.count_of(blue_gem) >= needed && session.inventory.can_add(white_gem)
        });

        if !ready {
            self.console(
                peer,
                &format!("You need {needed} Blue Gem Locks to convert."),
            );
            return;
        }

        let converted = self.sessions.get_mut(&peer).is_some_and(|session| {
            session
                .inventory
                .exchange(blue_gem, needed as u8, white_gem)
        });

        if converted {
            self.send_inventory(peer);
            self.console(
                peer,
                &format!("Converted {needed} Blue Gem Locks into a White Gem Lock."),
            );
        }
    }

    fn on_cheats_return(&mut self, peer: PeerId, text: &Text) {
        if let Some(name) = shop::cheat_button(text) {
            if let Some(session) = self.sessions.get_mut(&peer) {
                session.cheats.toggle(name);
            }
        }

        self.open_cheats(peer);
        self.send_character_state(peer);
    }

    fn open_telephone(&mut self, peer: PeerId) {
        let dialog = shop::dial_dialog();
        self.send_variant(peer, &dialog, framing::NET_ID_BROADCAST, 0);
    }

    fn open_equipment_shop(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else {
            return;
        };
        let dialog = shop::equipment_dialog(session.gems);
        self.send_variant(peer, &dialog, framing::NET_ID_BROADCAST, 0);
    }

    fn on_equipment_return(&mut self, peer: PeerId, text: &Text) {
        let Some(tool) = tools::CATALOG
            .iter()
            .find(|tool| shop::clicked(text, &format!("buy_tool_{}", tool.item)))
        else {
            return;
        };
        let bought = self.sessions.get_mut(&peer).is_some_and(|session| {
            tools::purchase(&mut session.inventory, &mut session.gems, tool.item)
        });
        if bought {
            self.send_inventory(peer);
            self.send_gems(peer);
            self.console(
                peer,
                &format!("Bought {} for {} gems.", tool.name, tool.price),
            );
        } else {
            self.console(peer, "Not enough gems or inventory space.");
        }
        self.open_equipment_shop(peer);
    }

    fn send_inventory(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else {
            return;
        };
        let payload = session.inventory.encode();
        let mut tank = TankPacket::new(TYPE_SEND_INVENTORY_STATE);
        tank.net_id = session.net_id;
        tank.set_extended(payload.len());
        self.send_tank(peer, &tank, &payload);
    }

    fn open_lock_shop(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else {
            return;
        };
        let dialog = shop::salesman_dialog(
            session.gems,
            session.tax_credits,
            session.growtokens,
            session.spray_tokens,
        );
        self.send_variant(peer, &dialog, framing::NET_ID_BROADCAST, 0);
    }

    fn open_tax(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else {
            return;
        };
        let dialog = shop::tax_dialog(
            session.tax_credits,
            session.growtokens,
            session.spray_tokens,
        );
        self.send_variant(peer, &dialog, framing::NET_ID_BROADCAST, 0);
    }

    fn open_spray(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else {
            return;
        };
        let dialog = shop::spray_dialog(session.spray_tokens);
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

    fn send_tokens(&mut self, peer: PeerId) {
        let Some(tokens) = self.sessions.get(&peer).map(|session| session.growtokens) else {
            return;
        };

        let list = VariantList::new("OnSetGrowTokens").arg(Variant::Int(tokens as i32));
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

        let generated = !self.worlds.is_loaded(world_name);
        self.worlds.ensure(world_name);

        if generated {
            logging::info(format!("generated world {world_name}"));
        }

        let width = self
            .worlds
            .get(world_name)
            .map_or(gtps_world::WIDTH, |world| world.width);
        let height = self.worlds.get(world_name).map_or(gtps_world::HEIGHT, |world| world.height);
        let (spawn_x, spawn_y) = world::spawn_position(width, gtps_world::world::DOOR_ROW.min(height.saturating_sub(1)));
        let net_id = allocate(&mut self.next_net_id) as i32;

        let Some(session) = self.sessions.get_mut(&peer) else {
            return;
        };

        session.world = Some(world_name.to_string());
        session.net_id = net_id;
        session.pos_x = spawn_x;
        session.pos_y = spawn_y;
        session.stage = Stage::InGame;

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
        self.send_character_state(peer);
    }

    fn on_tank(&mut self, peer: PeerId, tank: &TankPacket, _extra: &[u8]) {
        if !self
            .sessions
            .get(&peer)
            .is_some_and(|session| session.stage == Stage::InGame)
        {
            return;
        }
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

        if tools::by_item(item).is_some_and(|tool| tool.reach > 0) || matches!(item, gtps_world::items::FIST | tools::PICKAXE) {
            if let Some(session) = self.sessions.get_mut(&peer) {
                if session.inventory.count_of(item) > 0 {
                    session.equipped_hand = item;
                    session.remote_hits = 0;
                    self.console(peer, "Farming tool equipped.");
                }
            }
            self.send_character_state(peer);
            return;
        }

        if item == tools::RIPPER_WINGS {
            if let Some(session) = self.sessions.get_mut(&peer) {
                if session.inventory.count_of(item) > 0 {
                    session.equipped_back = if session.equipped_back == item { 0 } else { item };
                }
            }
            self.send_character_state(peer);
            return;
        }

        if item == locks::TELEPHONE_ITEM {
            self.open_telephone(peer);
            return;
        }

        if item == economy::ULTRA_WORLD_SPRAY {
            self.use_ultra_spray(peer);
            return;
        }

        if let Some(blast) = blast::by_item(item) {
            self.trigger_blast(peer, blast);
        }
    }

    fn trigger_blast(&mut self, peer: PeerId, blast: &blast::Blast) {
        let consumed = self
            .sessions
            .get_mut(&peer)
            .is_some_and(|session| session.inventory.remove(blast.item_id, 1));

        if !consumed {
            return;
        }

        let name = loop {
            let name = blast::world_name(allocate(&mut self.next_blast));
            if !self.worlds.is_loaded(&name) {
                break name;
            }
        };
        self.worlds.ensure_sized(&name, blast.width, blast.height);
        self.enter_world(peer, &name);
        self.console(
            peer,
            &format!(
                "{} opened a {}x{} world: {name}.",
                blast.name, blast.width, blast.height
            ),
        );
    }

    fn use_ultra_spray(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        let Some(world_name) = session.world.clone() else {
            return;
        };

        let Some(world) = self.worlds.get(&world_name) else {
            return;
        };
        let now = farming::now();
        if (world.owner_user_id != 0 && world.owner_user_id != session.user_id)
            || !world
                .tiles
                .iter()
                .any(|tile| tile.tree.is_some_and(|tree| !tree.mature(now)))
        {
            self.console(peer, "No growing trees here, or you do not own this world.");
            return;
        }
        let consumed = self
            .sessions
            .get_mut(&peer)
            .is_some_and(|stored| stored.inventory.remove(economy::ULTRA_WORLD_SPRAY, 1));

        if !consumed {
            return;
        }

        let mut grown: Vec<(i32, i32, Vec<u8>)> = Vec::new();

        if let Some(world) = self.worlds.get_mut(&world_name) {
            for index in farming::grow_all(world, now) {
                let Some((x, y)) = world.coords(index) else {
                    continue;
                };
                let Some(tile) = world.tile(x, y).copied() else {
                    continue;
                };
                grown.push((x as i32, y as i32, gtps_world::codec::encode_tile(&tile)));
            }
        }

        let targets = self.world_peers(&world_name, None);
        for (x, y, payload) in &grown {
            let mut update = TankPacket::new(TYPE_SEND_TILE_UPDATE_DATA);
            update.net_id = session.net_id;
            update.tile_x = *x;
            update.tile_y = *y;
            update.set_extended(payload.len());

            for target in &targets {
                self.send_tank(*target, &update, payload);
            }
        }

        self.send_inventory(peer);
        self.console(
            peer,
            &format!("Ultra World Spray grew {} plants.", grown.len()),
        );
    }

    fn broadcast_state(&mut self, peer: PeerId, tank: &TankPacket) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        if session.frozen { return; }
        let Some(world_name) = session.world.clone() else {
            return;
        };

        let Some(world) = self.worlds.get(&world_name) else {
            return;
        };
        if !tank.pos_x.is_finite()
            || !tank.pos_y.is_finite()
            || tank.pos_x < 0.0
            || tank.pos_y < 0.0
            || tank.pos_x >= world.width as f32 * 32.0
            || tank.pos_y >= world.height as f32 * 32.0
        {
            return;
        }
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
        if session.frozen { return; }
        let Some(world_name) = session.world.clone() else {
            return;
        };
        let (Ok(x), Ok(y)) = (usize::try_from(tank.tile_x), usize::try_from(tank.tile_y)) else {
            return;
        };

        let Ok(item) = u16::try_from(tank.int_data) else {
            return;
        };
        let Some((origin_x, origin_y, range)) = self.farm_reach(&session) else {
            return;
        };
        if origin_x.abs_diff(x) > range || origin_y.abs_diff(y) > range {
            return;
        }
        let Some(world) = self.worlds.get(&world_name) else {
            return;
        };
        let Some(index) = world.index(x, y) else {
            return;
        };
        let Some(old) = world.tile(x, y).copied() else {
            return;
        };
        if item == gtps_world::items::WRENCH {
            if old.foreground == locks::TELEPHONE_ITEM {
                self.open_telephone(peer);
            } else if old.foreground == tools::MAGPLANT {
                self.open_magplant(peer,index);
            }
            return;
        }
        if world.owner_user_id != 0 && world.owner_user_id != session.user_id && session.role != admin::Role::Owner {
            return;
        }
        if old.foreground == gtps_world::items::BEDROCK
            || old.foreground == gtps_world::items::MAIN_DOOR
        {
            return;
        }
        let mut reward = 0;
        let tile = if item == gtps_world::items::FIST {
            if old.foreground == 0 {
                return;
            }
            if session
                .last_punch
                .is_some_and(|time| time.elapsed() < Duration::from_millis(100))
            {
                return;
            }
            if let Some(stored) = self.sessions.get_mut(&peer) {
                stored.last_punch = Some(Instant::now());
            }
            let farmable = economy::table()
                .iter()
                .find(|farm| farm.block_id == old.foreground || farm.seed_id == old.foreground);
            if let Some(tree) = old.tree {
                if !tree.mature(farming::now()) {
                    return;
                }
                let Some(farm) = farmable else {
                    return;
                };
                if !acquisition::allows_world_reward(farm.block_id) {
                    return;
                }
                if let Some(stored) = self.sessions.get_mut(&peer) {
                    if !stored.inventory.add(farm.block_id, 1) {
                        return;
                    }
                }
                reward = economy::payout(farm, economy::Yield::Harvest, 100, self.next_rand());
            } else {
                let required = tools::hits(
                    &session.inventory,
                    session.equipped_hand,
                    farmable.map_or_else(|| self.items.as_ref().and_then(|db| db.get(old.foreground)).map_or(5, |item| item.hits), |farm| farm.hits),
                );
                let entry = self.worlds.entry_mut(&world_name);
                let damage = entry.damage.entry(index).or_default();
                *damage += 1;
                if *damage < required {
                    return;
                }
                entry.damage.remove(&index);
                if let Some(farm) = farmable {
                    if !self.sessions.get_mut(&peer).is_some_and(|stored| stored.inventory.add(farm.seed_id,1)) { return; }
                    reward = economy::payout(farm, economy::Yield::Break, 100, self.next_rand());
                }
            }
            gtps_world::Tile::new(0, old.background)
        } else {
            if old.foreground != 0
                || (tools::shop_only(item) && item != tools::MAGPLANT)
                || acquisition::exchange_only(item)
                || blast::by_item(item).is_some()
                || item == economy::ULTRA_WORLD_SPRAY
            {
                return;
            }
            if locks::by_item(item).is_some() && world.main_lock >= 0 {
                return;
            }
            let definition = self.items.as_ref().and_then(|db| db.get(item));
            let is_seed = definition.is_some_and(|item| item.kind == 19);
            if definition.is_some_and(|definition| definition.kind == 20) || (is_seed && item != economy::highest().seed_id) { return; }
            if is_seed && (!farming::can_plant(world, x, y) || !world.tile(x,y+1).and_then(|tile| self.items.as_ref()?.get(tile.foreground)).is_some_and(|item| matches!(item.collision,1 | 2))) {
                self.console(peer, "Seeds need a block directly beneath them.");
                return;
            }
            let taken = self
                .sessions
                .get_mut(&peer)
                .is_some_and(|stored| stored.inventory.remove(item, 1));
            if !taken {
                return;
            }
            match economy::table().iter().find(|farm| farm.seed_id == item) {
                Some(farm) => {
                    farming::plant(item, old.background, farm.grow_seconds, farming::now())
                }
                None if is_seed => farming::plant(item, old.background, definition.map_or(60, |item| item.grow_seconds), farming::now()),
                None => gtps_world::Tile::new(item, old.background),
            }
        };
        let entry = self.worlds.entry_mut(&world_name);
        entry.damage.remove(&index);
        entry.world.set_tile(x, y, tile);
        if locks::by_item(item).is_some() {
            entry.world.owner_user_id = session.user_id;
            entry.world.main_lock = index as i32;
        } else if item == gtps_world::items::FIST && entry.world.main_lock == index as i32 {
            entry.world.owner_user_id = 0;
            entry.world.main_lock = -1;
            entry.magplants.clear();
        }
        let lock_item = usize::try_from(entry.world.main_lock)
            .ok()
            .and_then(|tile| entry.world.tile_at(tile))
            .map_or(242, |tile| tile.foreground);
        if item == tools::MAGPLANT {
            self.worlds.ensure_magplant(&world_name, index, lock_item);
        } else if item == gtps_world::items::FIST {
            self.worlds.remove_magplant(&world_name, index);
        }
        if reward > 0 {
            self.award_gems(peer,reward);
        }
        self.send_inventory(peer);
        let payload = gtps_world::codec::encode_tile(&tile);

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
        if self.collect_gem(peer,uid) { return; }
        let Some(world) = self.worlds.get_mut(&world_name) else {
            return;
        };
        let Some(stored) = self.sessions.get_mut(&peer) else {
            return;
        };
        if !acquisition::collect(
            world,
            &mut stored.inventory,
            uid,
            session.pos_x,
            session.pos_y,
        ) {
            return;
        }
        self.send_inventory(peer);

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
        let peers: Vec<_> = self.sessions.keys().copied().collect();
        for peer in peers {
            self.auto_collect_gems(peer);
            self.flush_gem_bubble(peer);
        }
    }

    fn run_auto_farm(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer).cloned() else {
            return;
        };
        if session.frozen { return; }
        let Some(world_name) = session.world.clone() else {
            return;
        };

        if session
            .last_punch
            .is_some_and(|time| time.elapsed() < Duration::from_millis(100))
        {
            return;
        }
        let roll = self.next_rand();
        let mut reward = None;

        if session.cheats.bfg_mode {
            let reach = self.farm_reach(&session);
            let hits = tools::hits(
                &session.inventory,
                session.equipped_hand,
                economy::highest().hits,
            );
            if let Some(stored) = self.sessions.get_mut(&peer) {
                stored.remote_hits += 1;
                stored.last_punch = Some(Instant::now());
                if stored.remote_hits < hits {
                    return;
                }
                stored.remote_hits = 0;
            }
            let taken = self.worlds.take_from_magplant(
                &world_name,
                session.active_magplant,
                session.cheats.auto_grab_remote,
                1,
                reach,
            );

            if let Some((tile, lock_item, count)) = taken {
                let multiplier = locks::multiplier_percent(lock_item);
                let per_cycle = economy::cycle_gems(economy::highest(), multiplier, roll);

                if let Some(stored) = self.sessions.get_mut(&peer) {
                    stored.active_magplant = Some(tile);
                }

                reward = Some(per_cycle * count);
            } else if let Some(stored) = self.sessions.get_mut(&peer) {
                stored.last_punch = session.last_punch;
            }
        }

        if reward.is_none() && (session.cheats.use_blocks || session.cheats.bfg_mode) {
            self.punch_placed_block(peer);
        }

        if let Some(gems) = reward.filter(|gems| *gems > 0) {
            self.award_gems(peer,gems);
        }
    }

    fn punch_placed_block(&mut self, peer: PeerId) {
        let Some(session) = self.sessions.get(&peer) else {
            return;
        };
        let Some((x, y, range)) = self.farm_reach(session) else {
            return;
        };
        let Some(world) = session
            .world
            .as_ref()
            .and_then(|name| self.worlds.get(name))
        else {
            return;
        };
        let target = world.tiles.iter().enumerate().find_map(|(index, tile)| {
            let (tx, ty) = world.coords(index)?;
            if x.abs_diff(tx) > range || y.abs_diff(ty) > range {
                return None;
            }
            let farmable = economy::table()
                .iter()
                .any(|farm| farm.block_id == tile.foreground || farm.seed_id == tile.foreground);
            if !farmable || tile.tree.is_some_and(|tree| !tree.mature(farming::now())) {
                return None;
            }
            Some((tx, ty))
        });
        if let Some((tx, ty)) = target {
            let mut tank = TankPacket::new(TYPE_TILE_CHANGE_REQUEST);
            tank.int_data = i32::from(gtps_world::items::FIST);
            tank.tile_x = tx as i32;
            tank.tile_y = ty as i32;
            self.on_tile_change(peer, &tank);
        }
    }

    fn farm_reach(&self, session: &Session) -> Option<(usize, usize, usize)> {
        session.world.as_ref()?;

        let range = tools::reach(&session.inventory, session.equipped_hand);

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
            stored.remote_hits = 0;
            stored.dialog = None;
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
        if let Some(Variant::String(body)) = list.args().first() {
            if let Some(name) = body.lines().find_map(|line| {
                line.strip_prefix("end_dialog|")
                    .and_then(|line| line.split('|').next())
            }) {
                let Some(session) = self.sessions.get_mut(&peer) else {
                    return;
                };
                session.dialog_sequence = session.dialog_sequence.saturating_add(1);
                let nonce = session.dialog_sequence;
                session.dialog = Some((name.to_string(), nonce));
                let body = format!("add_embed_data|nonce|{nonce}\n{body}");
                let dialog = VariantList::new("OnDialogRequest").arg(Variant::String(body));
                self.send_raw(peer, framing::variant_packet(&dialog, net_id, delay));
                return;
            }
        }
        let frame = framing::variant_packet(list, net_id, delay);
        self.send_raw(peer, frame);
    }

    fn send_tank(&mut self, peer: PeerId, tank: &TankPacket, extra: &[u8]) {
        let frame = framing::game_packet(tank, extra);
        self.send_raw(peer, frame);
    }
}
