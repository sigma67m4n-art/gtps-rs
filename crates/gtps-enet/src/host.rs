use core::ffi::c_void;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::packet::Packet;
use crate::sys;
use crate::{Error, PeerId, PeerState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Server,
    Client,
}

#[derive(Debug)]
pub enum Event {
    Connect { peer: PeerId, data: u32 },
    Disconnect { peer: PeerId, data: u32 },
    Receive { peer: PeerId, channel: u8, packet: Packet },
}

pub struct Host {
    raw: *mut sys::ENetHost,
    role: Role,
    connected: HashSet<PeerId>,
}

unsafe impl Send for Host {}

fn to_enet_address(address: SocketAddr) -> Result<sys::ENetAddress, Error> {
    let ip = match address.ip() {
        IpAddr::V4(v4) => v4,
        IpAddr::V6(_) => return Err(Error::new("enet supports ipv4 only")),
    };
    let text = CString::new(ip.to_string()).map_err(|_| Error::new("invalid ip address"))?;

    let mut out = sys::ENetAddress {
        host: 0,
        port: address.port(),
    };

    if unsafe { sys::gtps_enet_address_set_ip(&mut out, text.as_ptr()) } != 0 {
        return Err(Error::new("enet_address_set_host_ip failed"));
    }

    unsafe { sys::gtps_enet_address_set_port(&mut out, address.port()) };
    Ok(out)
}

impl Host {
    fn create(role: Role, bind: SocketAddr, peers: usize, channels: usize) -> Result<Self, Error> {
        let address = to_enet_address(bind)?;

        let raw = unsafe {
            match role {
                Role::Server => sys::gtps_enet_host_create_server(&address, peers, channels),
                Role::Client => sys::gtps_enet_host_create_client(&address, peers, channels),
            }
        };

        if raw.is_null() {
            return Err(Error::new("enet_host_create failed"));
        }

        Ok(Self {
            raw,
            role,
            connected: HashSet::new(),
        })
    }

    pub fn server(bind: SocketAddr, peers: usize, channels: usize) -> Result<Self, Error> {
        Self::create(Role::Server, bind, peers, channels)
    }

    pub fn client(bind: SocketAddr, peers: usize, channels: usize) -> Result<Self, Error> {
        Self::create(Role::Client, bind, peers, channels)
    }

    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    pub fn connect(
        &mut self,
        target: SocketAddr,
        channels: usize,
        data: u32,
    ) -> Result<PeerId, Error> {
        let address = to_enet_address(target)?;
        let peer = unsafe { sys::enet_host_connect(self.raw, &address, channels, data) };

        if peer.is_null() {
            return Err(Error::new("no peer slots available"));
        }

        Ok(self.peer_id(peer))
    }

    pub fn service(&mut self, timeout_ms: u32) -> Result<Option<Event>, Error> {
        let mut buffer = [0u64; sys::EVENT_BUFFER_SIZE / 8];
        let event = buffer.as_mut_ptr().cast::<c_void>();

        let rc = unsafe { sys::gtps_enet_host_service(self.raw, event, timeout_ms) };
        if rc < 0 {
            return Err(Error::new("enet_host_service failed"));
        }
        if rc == 0 {
            return Ok(None);
        }

        let data = unsafe { sys::gtps_enet_event_data(event) };
        let peer = self.peer_id(unsafe { sys::gtps_enet_event_peer(event) });

        match unsafe { sys::gtps_enet_event_type(event) } {
            sys::EVENT_TYPE_CONNECT => {
                self.connected.insert(peer);
                Ok(Some(Event::Connect { peer, data }))
            }
            sys::EVENT_TYPE_DISCONNECT => {
                self.connected.remove(&peer);
                Ok(Some(Event::Disconnect { peer, data }))
            }
            sys::EVENT_TYPE_RECEIVE => {
                let channel = unsafe { sys::gtps_enet_event_channel(event) };
                let raw = unsafe { sys::gtps_enet_event_packet(event) };
                let packet = if raw.is_null() {
                    Packet::reliable([])
                } else {
                    Packet::from_raw(raw)
                };
                Ok(Some(Event::Receive {
                    peer,
                    channel,
                    packet,
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn send(&mut self, peer: PeerId, channel: u8, packet: Packet) -> Result<(), Error> {
        let raw = self.raw_peer(peer).ok_or_else(|| Error::new("unknown peer"))?;
        let raw_packet = packet.into_raw();

        if unsafe { sys::gtps_enet_peer_send(raw, channel, raw_packet) } != 0 {
            unsafe { sys::gtps_enet_packet_destroy(raw_packet) };
            return Err(Error::new("enet_peer_send failed"));
        }

        Ok(())
    }

    pub fn flush(&mut self) {
        unsafe { sys::gtps_enet_host_flush(self.raw) };
    }

    #[must_use]
    pub fn connected_peers(&self) -> usize {
        self.connected.len()
    }

    #[must_use]
    pub fn peer_state(&self, peer: PeerId) -> Option<PeerState> {
        let raw = self.raw_peer(peer)?;
        Some(PeerState::from_raw(unsafe {
            sys::gtps_enet_peer_state(raw)
        }))
    }

    #[must_use]
    pub fn peer_address(&self, peer: PeerId) -> Option<SocketAddr> {
        let raw = self.raw_peer(peer)?;
        let mut buffer = [0i8; 64];

        if unsafe { sys::gtps_enet_peer_ip(raw, buffer.as_mut_ptr(), buffer.len()) } != 0 {
            return None;
        }

        let text = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy();
        let ip: Ipv4Addr = text.parse().ok()?;
        let port = unsafe { sys::gtps_enet_peer_port(raw) };
        Some(SocketAddr::new(IpAddr::V4(ip), port))
    }

    #[must_use]
    pub fn peer_rtt_ms(&self, peer: PeerId) -> Option<u32> {
        let raw = self.raw_peer(peer)?;
        Some(unsafe { sys::gtps_enet_peer_rtt(raw) })
    }

    pub fn disconnect(&mut self, peer: PeerId, data: u32) {
        if let Some(raw) = self.raw_peer(peer) {
            unsafe { sys::gtps_enet_peer_disconnect(raw, data) };
        }
    }

    pub fn disconnect_now(&mut self, peer: PeerId, data: u32) {
        self.connected.remove(&peer);
        if let Some(raw) = self.raw_peer(peer) {
            unsafe { sys::gtps_enet_peer_disconnect_now(raw, data) };
        }
    }

    pub fn reset(&mut self, peer: PeerId) {
        self.connected.remove(&peer);
        if let Some(raw) = self.raw_peer(peer) {
            unsafe { sys::gtps_enet_peer_reset(raw) };
        }
    }

    fn raw_peer(&self, peer: PeerId) -> Option<*mut sys::ENetPeer> {
        let raw = unsafe { sys::gtps_enet_host_peer(self.raw, peer.0 as u32) };
        (!raw.is_null()).then_some(raw)
    }

    fn peer_id(&self, peer: *mut sys::ENetPeer) -> PeerId {
        let index = unsafe { sys::gtps_enet_peer_index(self.raw, peer) };
        PeerId(index.max(0) as usize)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sys::enet_host_destroy(self.raw) };
        }
    }
}
