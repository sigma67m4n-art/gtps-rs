# ENet patches

`vendor/enet` is ENet 1.3.18 taken from
`GTPSHAX/how-to-fix-growtopia-5.15`, trimmed to the sources `build.rs` compiles
(`callbacks.c`, `compress.c`, `host.c`, `list.c`, `packet.c`, `peer.c`,
`protocol.c`, `unix.c`, `win32.c` and the public headers).

Upstream comments inside these files are left untouched so the diff against the
fork stays readable.

The fork already carries the Growtopia "new packet" receive path. Two edits were
added on top:

1. `include/enet/enet.h` — `ENetHost` now has both flags:

   ```c
   enet_uint8 usingNewPacket;
   enet_uint8 usingNewPacketForServer;
   ```

   The receive path in `protocol.c` keys off `usingNewPacketForServer`; the send
   path added below keys off `usingNewPacket`.

2. `protocol.c` — `enet_protocol_send_outgoing_commands` sizes the header from
   `headerSize` (10 bytes for `ENetNewProtocolHeader`, 4 for the legacy one),
   fills the three `integrity` words plus `peerID` and `sentTime` when
   `usingNewPacket` is set, and uses `headerSize` in the compression math. The
   integrity words are built from `gtps_enet_rand`, an xorshift32 defined in
   `crates/gtps-enet/src/shim.c`:

   ```
   integrity[0] = rand() % (port + 1)
   integrity[1] = integrity[0] ^ port
   integrity[2] = (rand() & 0x61D2) | 0x920D
   ```

`crates/gtps-enet/src/shim.c` also sets `checksum = enet_crc32` and installs the
range coder on both host roles, because a Growtopia client silently drops hosts
that do not.

The send path writes `sentTime` through the header selected for that host role.
Writing it at the new-header offset for server packets left the legacy timestamp
uninitialized, causing unreliable acknowledgment timing during large transfers.
