use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use gtps_enet::{Event, Host, Packet};

fn free_udp_port() -> u16 {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind udp socket");
    socket.local_addr().expect("local addr").port()
}

#[test]
fn client_and_server_exchange_a_reliable_packet() {
    gtps_enet::initialize().expect("enet initialize");

    let server_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, free_udp_port()));
    let client_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, free_udp_port()));

    let mut server = Host::server(server_addr, 8, 2).expect("server host");
    let mut client = Host::client(client_addr, 1, 2).expect("client host");

    client
        .connect(server_addr, 2, 0x1122_3344)
        .expect("connect");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut server_peer = None;

    while Instant::now() < deadline && server_peer.is_none() {
        if let Some(Event::Connect { peer, .. }) = server.service(100).expect("server service") {
            server_peer = Some(peer);
        }

        let _ = client.service(10);
    }

    let server_peer = server_peer.expect("server never saw the connect event");

    server
        .send(server_peer, 0, Packet::reliable(b"hello from the server"))
        .expect("server send");
    server.flush();

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = None;

    while Instant::now() < deadline && received.is_none() {
        if let Some(Event::Receive { packet, .. }) = client.service(100).expect("client service") {
            received = Some(packet.data().to_vec());
        }
    }

    assert_eq!(
        received.as_deref(),
        Some(b"hello from the server".as_slice())
    );
}
