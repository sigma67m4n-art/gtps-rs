use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::config::Config;
use crate::logging;

pub const END_MARKER: &str = "RTENDMARKERBS1001";

pub fn spawn(config: &Config) -> std::io::Result<()> {
    let listener = TcpListener::bind(config.http_bind)?;
    logging::info(format!("http listening on {}", config.http_bind));

    let config = config.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle(stream, &config),
                Err(error) => logging::warn(format!("http accept failed: {error}")),
            }
        }
    });

    Ok(())
}

fn handle(mut stream: TcpStream, config: &Config) {
    let mut buffer = [0u8; 2048];
    let _ = stream.read(&mut buffer);

    let body = server_data(config);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[must_use]
pub fn server_data(config: &Config) -> String {
    let ip = config.game_bind.ip();
    let port = config.game_bind.port();

    format!(
        "server|{ip}\nport|{port}\ntype|1\ntype2|1\nmaint|0\nbeta_server|\nbeta_port|0\nbeta_type|0\nmeta|\n{END_MARKER}"
    )
}
