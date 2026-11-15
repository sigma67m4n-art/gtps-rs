use std::env;
use std::net::SocketAddr;

use crate::logging::Level;

#[derive(Debug, Clone)]
pub struct Config {
    pub game_bind: SocketAddr,
    pub http_bind: SocketAddr,
    pub server_name: String,
    pub motd: String,
    pub cdn_host: String,
    pub max_peers: usize,
    pub channels: usize,
    pub items_path: String,
    pub log_level: Level,
    pub starting_gems: u32,
    pub inventory_capacity: u32,
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
            cdn_host: "ubistatic-a.akamaihd.net".to_string(),
            max_peers: 1024,
            channels: 2,
            items_path: "data/items.dat".to_string(),
            log_level: Level::Info,
            starting_gems: 100,
            inventory_capacity: 596,
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
            max_peers: env_parsed("GTPS_MAX_PEERS", default.max_peers),
            channels: env_parsed("GTPS_CHANNELS", default.channels),
            items_path: env_string("GTPS_ITEMS_PATH", &default.items_path),
            log_level: Level::from_name(&env_string("GTPS_LOG_LEVEL", "info")),
            starting_gems: env_parsed("GTPS_STARTING_GEMS", default.starting_gems),
            inventory_capacity: env_parsed("GTPS_INVENTORY_CAPACITY", default.inventory_capacity),
        }
    }
}
