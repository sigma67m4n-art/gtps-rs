# gtps-rs

Rust Growtopia server with ENet transport, account authentication, a custom
Gems Store, Pot O' Gold farming, and Owner administration. GitHub Actions
builds, tests and lints the workspace and produces x86_64 and static ARM64 binaries.

| Crate | Purpose |
| --- | --- |
| gtps-auth | Argon2id credentials, SQLite accounts, roles, single-use tokens and snapshots |
| gtps-enet | Vendored ENet and Rust transport wrapper |
| gtps-proto | Packet codecs and validated items.dat metadata |
| gtps-world | World generation, tiles and serialization |
| gtps-server | HTTP login, game sessions and gameplay |

## Starting the server

Run the GitHub-built `gtps-server` from this repository directory. The included
`data/items.dat` contains 16,434 vanilla items in format v27. Source and checksum
are recorded in `data/ITEMS.md`; the source was updated on October 15, 2026.
Every record is validated at startup. The served database changes stack limits
of 200 to 250 and configures Pot O' Gold as a six-hit foreground block.

| Variable | Default |
| --- | --- |
| GTPS_GAME_BIND | 0.0.0.0:17091 |
| GTPS_HTTP_BIND | 0.0.0.0:8080 |
| GTPS_PUBLIC_HOST | 127.0.0.1 |
| GTPS_LOGIN_URL | http://127.0.0.1:8080 |
| GTPS_AUTH_PATH | data/accounts.sqlite |
| GTPS_ITEMS_PATH | data/items.dat |
| GTPS_OWNER_KEY | empty; Owner bootstrap disabled |
| GTPS_SERVER_NAME | gtps-rs |
| GTPS_MOTD | Welcome to gtps-rs |
| GTPS_STARTING_GEMS | 100 |
| GTPS_INVENTORY_CAPACITY | 596 |
| GTPS_MAX_PEERS | 1024 |
| GTPS_CHANNELS | 2 |
| GTPS_LOG_LEVEL | info |
| GTPS_CDN_HOST | 127.0.0.1:8080 |
| GTPS_CDN_PATH | game/ |
| GTPS_VANILLA_CDN_URL | https://ubistatic-a.akamaihd.net/0098/15230125103/cache/game/ |

## Accounts

Open `/player/login/dashboard` to register or log in. GrowIDs accept 3–18 ASCII
letters, numbers or underscores; passwords accept 10–128 bytes. Login endpoints
also accept JSON with `growId` and `password`; registration additionally needs
`confirmPassword`. GrowServer is used only as a reference for the login flow.

Routes include `/player/growid/login/validate`, `/player/growid/register/validate`,
`/player/growid/checktoken`, `/player/growid/validate/checktoken/<token>`,
`/growtopia/server_data.php` and `/health`. Browser forms redirect to the validated
token route. JSON clients receive the login payload directly. Tokens expire in
five minutes and are consumed by ENet login. Legacy GrowID/password login is
verified against the same account database. Guest names do not authenticate.
Concurrent game sessions for the same account are rejected.

Credentials use salted Argon2id. Token hashes, account roles, bans, inventories,
balances, worlds, trees and Magplant stock persist in SQLite. Player and world
snapshots commit together after game events and farming ticks. HTTP login has
request-size and per-address rate limits. TLS termination and retail client
host routing require a reverse proxy; the built-in listener uses HTTP.

## Starter pack and farming

A new account receives **250 Pot O' Gold (`542`), 250 Pot O' Gold Seeds (`543`),
250 Wooden Platforms (`102`), one Ultra World Spray (`12600`), one Ripper Wings
(`818`), one Pickaxe (`98`), one My First World Lock (`9640`), Fist and Wrench**
on its first game login. My First World Lock claims world ownership and uses the
WL tier (5,000 Magplant stock, 1× gems). Reconnecting restores
the saved inventory. Inventory stacks hold at most **250** items.

Pot O' Gold is the only farmable. Breaking it takes six normal hits and pays
240 gems; harvesting its mature tree pays 156 gems (65%). Growth takes four
minutes. Breaking supplies a seed; harvesting supplies a block. Seeds require
a supporting collision tile beneath them, checked before inventory consumption.
Other blocks use the normal hardness from items.dat and do not give farming
rewards. Pickaxe reduces normal hit counts and has one-tile reach; one-hit
shop tools retain their own effects.

## Gems Store

Use the client's **Gems/Store button** (`action|store`). There is no `/shop`
command. Add equipment entries in `game/tools.rs` to extend the catalog.

| Equipment | Gems | Reach | Hits |
| --- | --- | --- | --- |
| Rayman's Fist | 50,000 | 3 | normal |
| Death's Scythe | 250,000 | 1 | 1 |
| Legendary Infinity Rayman's Fist | 25,000,000 | 7 | 1 |
| Mythical Infinity Rayman's Fist | 750,000,000 | 10 | 1 |
| MAGPLANT 5000 | 75,000 | — | — |

Equipment prices have no extra tax. Effects require ownership and equipping.
These items and their seeds cannot come from ground pickup, farming or `/give`.
Starter Pickaxe and Ripper Wings are separate vanilla equipment.

## Telephone

Activate or wrench Telephone (`3898`) and dial:

| Number | Service |
| --- | --- |
| 53785 | Sales-Man: DL 100,000 gems +5%; BGL 10,000,000 gems +5% |
| 12345 | 100,000 tax credit per Growtoken; 1,000,000 per Tourney Token |
| 53675 | Ultra World Spray (vanilla ID 12600) for five Tourney Tokens |

Sales-Man converts 100 DL to one BGL, or 100 BGL to one White Gem Lock.
Tourney Tokens (`4212`) are a service balance available only from tax exchange.
Growtokens and locks are exempt from that acquisition restriction. Spray matures
every growing tree in the current world when activated from inventory.

## Magplants, cheats and Owner

| World lock tier | Maximum stock | Gem multiplier |
| --- | --- | --- |
| WL | 5,000 | 1× |
| DL | 15,000 | 1× |
| BGL | 50,000 | 1× |
| Emerald | 100,000 | 1.5× |
| Royal | 200,000 | 2× |
| White Gem | 500,000 | 2.25× |

Magplants currently start with finite Pot O' Gold stock. Wrench a machine to get
its remote. Owners also see `Add <maximum> Stock`, which fills up to the cap.
Inventory deposits are not implemented.

`/cheats` offers Auto Farm, BFG Mode, Auto Grab Remote, Use Placed Blocks,
Super Speed, High Jump, Modfly and Auto Collect Gems. Remote farming consumes
stock only on the final required hit and can fall back to placed farmables.
Ground gem collection is limited to nearby objects. Collected gems are grouped
into one `Collected <total> Gems (gems)` bubble per 500 ms batch.

Use `/owner <GTPS_OWNER_KEY>` while authenticated to attach Owner to an account.
Names cannot grant roles. Moderators can use `/who`, `/goto`, `/summon`, `/kick`,
`/ban`, `/unban <account ID>`, `/mute`, `/unmute`, `/freeze` and `/unfreeze`.
Owner also has `/give <ID> <count>`, `/drop <ID> <count>`, `/setgems <amount>`,
`/setrole <online name> player|moderator`, `/clearworld`, `/cleardrop` and
`/createworld <width>x<height>` (10–200 per side). Drop consumes inventory;
count is bounded to 1–250. Role hierarchy prevents moderating higher roles.

Inventory transactions validate capacity before spending. Collection checks
distance before removing objects. Dialog actions need a matching single-use
nonce. Seed support, punch cooldown, world ownership and role checks run server-side.

## Custom client assets

The served database includes custom definitions and a bundled RTTEX atlas.
Legendary/Mythical artwork is extracted from the supplied Creative PS references;
White Gem Lock is a white recolor of the BGL icon. See `assets/preview.png` and
`assets/README.md` for provenance. Custom IDs are outside the vanilla range:

| Item | ID |
| --- | --- |
| Legendary Infinity Rayman's Fist | 60000 |
| Mythical Infinity Rayman's Fist | 60002 |
| White Gem Lock | 60004 |
| Small Blast (50×50) | 60008 |
| Large Blast (200×200) | 60010 |

Small/Large Blasts reuse the vanilla Thermonuclear Blast icon. Ultra World Spray
uses vanilla ID 12600 and its original texture. Reserved gaps cannot be given
or placed. Custom definitions are generated at startup; `data/items.dat` stays vanilla.

The HTTP listener serves `/game/tiles/custom-icons.rttex` and
`/game/custom-icons.rttex`; other `/game/` asset requests redirect to the configured
vanilla CDN. Set `GTPS_CDN_HOST` to the client-reachable asset host, and route
`/game/` to this listener through your HTTPS reverse proxy for retail clients.
`GTPS_CDN_PATH` defaults to `game/`; keep it aligned with the proxy routes.
Retail-client rendering, CDN redirect handling and movement effects still need
an actual-client validation pass. The reference images are compressed, so the
Infinity sprites are not verified pixel-identical copies of the original assets.
