mod config;
mod game;
mod http;
mod logging;
mod store;

use std::process::ExitCode;

fn main() -> ExitCode {
    let config = config::Config::from_env();
    logging::init(config.log_level);
    if let Some(parent) = std::path::Path::new(&config.auth_path).parent() {
        if !parent.as_os_str().is_empty() && std::fs::create_dir_all(parent).is_err() {
            logging::error("failed to create account data directory");
            return ExitCode::FAILURE;
        }
    }

    logging::info(format!(
        "gtps-rs starting: game {} http {}",
        config.game_bind, config.http_bind
    ));

    if let Err(error) = http::spawn(&config) {
        logging::error(format!("failed to start the http server: {error}"));
        return ExitCode::FAILURE;
    }

    let mut game = match game::Game::new(config) {
        Ok(game) => game,
        Err(error) => {
            logging::error(format!("failed to start the game server: {error}"));
            return ExitCode::FAILURE;
        }
    };

    match game.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            logging::error(format!("game loop stopped: {error}"));
            ExitCode::FAILURE
        }
    }
}
