use std::env;
use std::net::SocketAddr;

use crate::logging::Level;

#[derive(Clone)]
pub struct Config {
    pub game_bind: SocketAddr,
    pub http_bind: SocketAddr,
    pub server_name: String,
    pub motd: String,
    pub cdn_host: String,
    pub cdn_path: String,
    pub vanilla_cdn_url: String,
    pub max_peers: usize,
    pub channels: usize,
    pub items_path: String,
    pub log_level: Level,
    pub starting_gems: u32,
    pub inventory_capacity: u32,
    pub owner_key: String,
    pub auth_path: String,
    pub public_host: String,
    pub login_url: String,
}

fn env_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parsed<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_bind: SocketAddr::from(([0, 0, 0, 0], 17091)),
            http_bind: SocketAddr::from(([0, 0, 0, 0], 8080)),
            server_name: "gtps-rs".to_string(),
            motd: "Welcome to gtps-rs".to_string(),
            cdn_host: "127.0.0.1:8080".to_string(),
            cdn_path: "game/".to_string(),
            vanilla_cdn_url: "https://ubistatic-a.akamaihd.net/0098/15230125103/cache/game/"
                .to_string(),
            max_peers: 1024,
            channels: 2,
            items_path: "data/items.dat".to_string(),
            log_level: Level::Info,
            starting_gems: 100,
            inventory_capacity: 596,
            owner_key: String::new(),
            auth_path: "data/accounts.sqlite".into(),
            public_host: "127.0.0.1".into(),
            login_url: "http://127.0.0.1:8080".into(),
        }
    }
}

impl Config {
    #[must_use]
    pub fn from_env() -> Self {
        let default = Self::default();

        Self {
            game_bind: env_parsed("GTPS_GAME_BIND", default.game_bind),
            http_bind: env_parsed("GTPS_HTTP_BIND", default.http_bind),
            server_name: env_string("GTPS_SERVER_NAME", &default.server_name),
            motd: env_string("GTPS_MOTD", &default.motd),
            cdn_host: env_string("GTPS_CDN_HOST", &default.cdn_host),
            cdn_path: env_string("GTPS_CDN_PATH", &default.cdn_path),
            vanilla_cdn_url: env_string("GTPS_VANILLA_CDN_URL", &default.vanilla_cdn_url),
            max_peers: env_parsed("GTPS_MAX_PEERS", default.max_peers),
            channels: env_parsed("GTPS_CHANNELS", default.channels),
            items_path: env_string("GTPS_ITEMS_PATH", &default.items_path),
            log_level: Level::from_name(&env_string("GTPS_LOG_LEVEL", "info")),
            starting_gems: env_parsed("GTPS_STARTING_GEMS", default.starting_gems),
            inventory_capacity: env_parsed("GTPS_INVENTORY_CAPACITY", default.inventory_capacity),
            owner_key: env_string("GTPS_OWNER_KEY", ""),
            auth_path: env_string("GTPS_AUTH_PATH", &default.auth_path),
            public_host: env_string("GTPS_PUBLIC_HOST", &default.public_host),
            login_url: env_string("GTPS_LOGIN_URL", &default.login_url),
        }
    }
}
