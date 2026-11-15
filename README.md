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

## Farming

Farming has two actions: **harvest** and **break**. Harvesting pays a little
less, breaking a block pays more. For high tier farmables the harvest payout is
`65%` of the break payout; low tier farmables (Dirt, Rock, Cave Background) are
left untouched and pay the same either way.

Pot O' Gold is the best farmable: its block takes `5-6` hits (first hits are
harvests, the last hit is the break) and its payout is perfectly stable. The
table lives in `crates/gtps-server/src/game/economy.rs` and is easy to edit:

| farmable | block | break gems | hits | tier |
| --- | --- | --- | --- | --- |
| Pot O' Gold | 10340 | 240 | 6 | high |
| Lava | 10 | 160 | 1 | high |
| Sand | 20 | 125 | 1 | high |
| Wood Block | 22 | 95 | 1 | high |
| Brick | 26 | 70 | 1 | high |
| Glass Block | 24 | 50 | 1 | high |
| Dirt | 2 | 15 | 1 | low |
| Rock | 4 | 12 | 1 | low |
| Cave Background | 14 | 5 | 1 | low |

Item ids are best effort until an `items.dat` is available.

## Telephone service

Locks are no longer sold from the gem store. Use the telephone (item `53785`)
and dial the service number `53785` to reach the lock service:

| option | cost |
| --- | --- |
| Diamond Lock | 100,000 gems |
| Blue Gem Lock | 10,000,000 gems (+ 500,000 tax) |
| Convert to Blue Gem Lock | 100 Diamond Locks |

Diamond Locks can only be bought. Blue Gem Locks can be bought or converted
from `100` Diamond Locks. Tax is `5%` of every full `1,000,000` gems (`50,000`
per million).

## Magplants

Every lock placed in a world becomes a magplant with its own stock and gem
multiplier:

| lock | item | stock | gems |
| --- | --- | --- | --- |
| World Lock | 1796 | 5,000 | 1.0x |
| Diamond Lock | 7188 | 15,000 | 1.0x |
| Blue Gem Lock | 12886 | 50,000 | 1.0x |
| Emerald Lock | 13254 | 100,000 | 1.5x |
| Royal Lock | 8482 | 200,000 | 2.0x |

Lock ids are best effort until an `items.dat` is available.

## Cheats

Type `/cheats` or use the wrench to open the cheats menu. `/gems` prints the
current gem balance.

| option | effect |
| --- | --- |
| Auto Farm | farms the best farmable block you carry |
| Auto Farm: BFG Mode | farms from the remote magplant nearby |
| Auto Grab Magplant Remote | when a magplant runs dry, grab another one in range |
| Auto Farm: Use Placed Blocks | place blocks (from your inventory) instead of a magplant |

Auto farm reach is `1` tile with the fist and `3` tiles with Rayman's Fist
(item `13634`). Without a magplant, BFG Mode falls back to placing the blocks
you carry.

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
