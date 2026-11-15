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
crates/gtps-world    world model and variable-size serialization
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

Pot O' Gold is the best farmable: its block takes six normal hits and pays
only on the final hit. Harvest rewards require a mature tree. Its payout is stable. The
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

## Telephone services

Use the telephone (item `3898`) and dial a service number:

| number | service |
| --- | --- |
| `53785` | Sales-Man (locks) |
| `12345` | Tax service |
| `53675` | Spray shop |

### Sales-Man (`53785`)

| option | cost |
| --- | --- |
| Diamond Lock | 100,000 gems (+ 5,000 tax) |
| Blue Gem Lock | 10,000,000 gems (+ 500,000 tax) |
| Convert to Blue Gem Lock | 100 Diamond Locks |
| Convert to White Gem Lock | 100 Blue Gem Locks |

### Tax service (`12345`)

Tax is `5%` of every lock purchase and is kept as **tax credit**. Tax credit can
be converted into two tokens:

| token | tax credit per token |
| --- | --- |
| Growtoken | 100,000 |
| Tourney Token (vanilla item `4212`) | 1,000,000 |

A Tourney Token costs ten times as much tax credit as a Growtoken. Tourney
Tokens are held as a service balance, displayed with their vanilla icon, and
issued only by tax exchange. Pickup and planting cannot supply them. This
restriction does not apply to Growtokens or locks. Tournament rewards are not
implemented on this server.

### Spray shop (`53675`)

| item | cost | effect |
| --- | --- | --- |
| Ultra World Spray | 5 Tourney Tokens | instantly grows every plant in the world |

The Ultra World Spray is used from the inventory like fertilizer, but it grows
every planted seed in the current world at once.

## Blast worlds

Blast items are activated from the inventory and create a new empty world:

| item | world |
| --- | --- |
| Small Blast (`13620`) | new empty 50x50 world |
| Large Blast (`13621`) | new empty 200x200 world |

Each blast consumes one item, creates a fresh `BLAST<n>` world and moves the
player into it.

## Transactions

Purchases and conversions preserve balances when inventory additions fail.
Collection checks distance and inventory capacity before removing an object.
Dialog transactions require the currently open dialog and a single-use nonce.
Accounts, balances and worlds are in memory; durable authentication and
persistence are not implemented. These checks are not a complete exploit audit.

## Equipment shop

Open the client's store to buy equipment with gems. Prices include no extra tax.
Effects require owning and equipping the hand item.

| item | gems | reach | hits |
| --- | --- | --- | --- |
| Rayman's Fist (`5480`) | 50,000 | 3 tiles | normal |
| Death's Scythe (`4166`) | 250,000 | 1 tile | 1 |
| Legendary Rayman Fist (`60000`) | 25,000,000 | 7 tiles | 1 |
| Mythical Rayman Fist (`60002`) | 750,000,000 | 14 tiles | 1 |
| MAGPLANT 5000 (`5638`) | 75,000 | — | — |

These items can only be acquired through gem purchases. Their seed variants,
ground pickup and farming rewards are blocked. Crafting, trading and event
reward systems are not implemented. The bare fist has one-tile reach.

Custom item IDs are provisional. Legendary/Mythical must use Creative PS
assets; those assets and definitions have not been supplied. White Gem Lock
(`13685`) requires a white recolor of the BGL (`7188`) icon. Its conversion and
server effects exist, but its client appearance is not installed. Ultra World
Spray and both blasts also require matching client definitions. No custom
sprite pack or modified `items.dat` is included.

## Magplants

Placed MAGPLANT 5000 machines use the world's lock tier for stock and gem
multiplier. The current BFG prototype supplies a finite Pot O' Gold stock when
a new machine is placed; inventory deposits and machine configuration are not
implemented:

| lock | item | stock | gems |
| --- | --- | --- | --- |
| World Lock | 242 | 5,000 | 1.0x |
| Diamond Lock | 1796 | 15,000 | 1.0x |
| Blue Gem Lock | 7188 | 50,000 | 1.0x |
| Emerald Lock | 2408 | 100,000 | 1.5x |
| Royal Lock | 4802 | 200,000 | 2.0x |
| White Gem Lock | 13685 | 500,000 | 2.25x |

White Gem Lock is converted from 100 BGL at Sales-Man (`53785`).

## Cheats

Type `/cheats` or use the wrench to open the cheats menu. `/gems` prints the
current gem balance.

| option | effect |
| --- | --- |
| Auto Farm | enables automatic farming ticks |
| Auto Farm: BFG Mode | farms from the remote magplant nearby |
| Auto Grab Magplant Remote | when a magplant runs dry, grab another one in range |
| Auto Farm: Use Placed Blocks | punches reachable farmable blocks and mature trees |

Auto farm uses the equipped tool's hit count and reach. BFG rewards a block
break only after its required hits; intermediate hits do not award gems.
Without magplant stock it falls back to reachable placed farmables.

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
