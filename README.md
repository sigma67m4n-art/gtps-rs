# gtps-rs

A Growtopia private server written in Rust. It speaks the retail wire protocol
(ENet with the "new packet" header, `server_data.php`, tank packets and
variants) and is built as a four crate Cargo workspace.

| crate | role |
| --- | --- |
| `gtps-enet` | vendored ENet C sources plus a small safe-ish Rust wrapper |
| `gtps-proto` | wire codecs: cursors, text, variants, tank packets, items.dat |
| `gtps-world` | world model, generation and binary serialization |
| `gtps-server` | the server binary: http login, ENet loop, worlds, sessions |

Everything is built and tested by GitHub Actions; see
`.github/workflows/ci.yml`.

## Layout

```
crates/gtps-enet     ENet FFI (vendor/enet is compiled by build.rs)
crates/gtps-proto    protocol codecs
crates/gtps-world    100x60 world model
crates/gtps-server   binary
vendor/enet          patched ENet 1.3.18 (see vendor/enet/PATCHES.md)
data/                runtime data (items.dat goes here)
```

## Configuration

The server is configured through environment variables:

| variable | default | meaning |
| --- | --- | --- |
| `GTPS_GAME_BIND` | `0.0.0.0:17091` | ENet listen address |
| `GTPS_HTTP_BIND` | `0.0.0.0:8080` | http login listen address |
| `GTPS_SERVER_NAME` | `gtps-rs` | name shown in the world select menu |
| `GTPS_MOTD` | `Welcome to gtps-rs` | console message sent after login |
| `GTPS_CDN_HOST` | `ubistatic-a.akamaihd.net` | CDN host sent to the client |
| `GTPS_MAX_PEERS` | `1024` | maximum concurrent peers |
| `GTPS_CHANNELS` | `2` | ENet channels |
| `GTPS_ITEMS_PATH` | `data/items.dat` | item database path |
| `GTPS_LOG_LEVEL` | `info` | `error`, `warn`, `info` or `debug` |
| `GTPS_STARTING_GEMS` | `100` | gems given on first login |
| `GTPS_INVENTORY_CAPACITY` | `596` | inventory slot capacity |

`items.dat` is not shipped. Drop a copy at `data/items.dat`. When it is missing
the server still runs and simply ignores `refresh_item_data` requests.

## Login

The client posts to `/growtopia/server_data.php`; any path is answered with:

```
server|<ip>
port|<port>
type|1
type2|1
maint|0
beta_server|
beta_port|0
beta_type|0
meta|
RTENDMARKERBS1001
```

TLS is not terminated here. Put the server behind a reverse proxy (nginx, caddy)
or point the client at it through a hosts entry, since the client uses https.
