#![allow(dead_code)]
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct ENetHost {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ENetPeer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ENetPacket {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ENetAddress {
    pub host: u32,
    pub port: u16,
}

pub const EVENT_TYPE_CONNECT: c_int = 1;
pub const EVENT_TYPE_DISCONNECT: c_int = 2;
pub const EVENT_TYPE_RECEIVE: c_int = 3;

pub const PEER_STATE_CONNECTED: c_int = 5;
pub const PEER_STATE_DISCONNECTED: c_int = 0;

pub const PACKET_FLAG_RELIABLE: u32 = 1 << 0;

pub const EVENT_BUFFER_SIZE: usize = 128;

extern "C" {
    pub fn enet_initialize() -> c_int;
    pub fn enet_deinitialize();

    pub fn gtps_enet_host_create_server(
        address: *const ENetAddress,
        peers: usize,
        channels: usize,
    ) -> *mut ENetHost;
    pub fn gtps_enet_host_create_client(
        address: *const ENetAddress,
        peers: usize,
        channels: usize,
    ) -> *mut ENetHost;
    pub fn enet_host_destroy(host: *mut ENetHost);
    pub fn enet_host_flush(host: *mut ENetHost);
    pub fn enet_host_connect(
        host: *mut ENetHost,
        address: *const ENetAddress,
        channels: usize,
        data: u32,
    ) -> *mut ENetPeer;

    pub fn gtps_enet_host_service(host: *mut ENetHost, event: *mut c_void, timeout_ms: u32) -> c_int;
    pub fn gtps_enet_host_peer(host: *mut ENetHost, id: u32) -> *mut ENetPeer;
    pub fn gtps_enet_host_connected_peers(host: *const ENetHost) -> usize;

    pub fn gtps_enet_address_set_ip(address: *mut ENetAddress, ip: *const c_char) -> c_int;
    pub fn gtps_enet_address_set_port(address: *mut ENetAddress, port: u16);

    pub fn gtps_enet_event_type(event: *const c_void) -> c_int;
    pub fn gtps_enet_event_data(event: *const c_void) -> u32;
    pub fn gtps_enet_event_channel(event: *const c_void) -> u8;
    pub fn gtps_enet_event_peer(event: *const c_void) -> *mut ENetPeer;
    pub fn gtps_enet_event_packet(event: *const c_void) -> *mut ENetPacket;

    pub fn gtps_enet_peer_index(host: *const ENetHost, peer: *const ENetPeer) -> c_int;
    pub fn gtps_enet_peer_state(peer: *const ENetPeer) -> c_int;
    pub fn gtps_enet_peer_ip(peer: *const ENetPeer, buffer: *mut c_char, length: usize) -> c_int;
    pub fn gtps_enet_peer_port(peer: *const ENetPeer) -> u16;
    pub fn gtps_enet_peer_rtt(peer: *const ENetPeer) -> u32;
    pub fn gtps_enet_peer_send(peer: *mut ENetPeer, channel: u8, packet: *mut ENetPacket) -> c_int;
    pub fn gtps_enet_peer_disconnect(peer: *mut ENetPeer, data: u32);
    pub fn gtps_enet_peer_disconnect_now(peer: *mut ENetPeer, data: u32);
    pub fn gtps_enet_peer_reset(peer: *mut ENetPeer);

    pub fn enet_packet_create(data: *const c_void, length: usize, flags: u32) -> *mut ENetPacket;
    pub fn gtps_enet_packet_data(packet: *const ENetPacket) -> *const u8;
    pub fn gtps_enet_packet_size(packet: *const ENetPacket) -> usize;
    pub fn gtps_enet_packet_destroy(packet: *mut ENetPacket);
}
