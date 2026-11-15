use super::*;

fn pump(game: &mut Game, client: &mut Host, mut ready: impl FnMut(&Game, &[Message]) -> bool) -> Vec<Message> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut messages = Vec::new();
    loop {
        if let Some(event) = game.host.service(1).unwrap() {
            game.handle_event(event);
            game.checkpoint().unwrap();
        }
        game.host.flush();
        if let Some(Event::Receive { packet, .. }) = client.service(1).unwrap() {
            messages.push(Message::decode(packet.data()).unwrap());
        }
        client.flush();
        if ready(game, &messages) {
            return messages;
        }
        assert!(Instant::now() < deadline, "ENet gameplay test timed out");
    }
}

fn send(client: &mut Host, peer: PeerId, text: &str) {
    client.send(peer, 0, Packet::reliable(framing::generic_text(text))).unwrap();
    client.flush();
}

#[test]
fn enet_login_custom_database_store_and_starter_persistence() {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let config = Config {
        game_bind: address,
        auth_path: ":memory:".into(),
        items_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/items.dat").into(),
        ..Config::default()
    };
    let mut game = Game::new(config).unwrap();
    let account = game.auth.register("NetworkTest", "network-test-password").unwrap();
    let mut client = Host::client("127.0.0.1:0".parse().unwrap(), 1, 2).unwrap();
    let peer = client.connect(address, 2, 0).unwrap();
    pump(&mut game, &mut client, |_, messages| messages.iter().any(|message| matches!(message, Message::ServerHello)));
    send(&mut client, peer, "tankIDName|NetworkTest\ntankIDPass|network-test-password\n");
    pump(&mut game, &mut client, |game, _| game.sessions.values().any(|session| session.user_id == account.id));
    let server_peer = *game.sessions.keys().next().unwrap();
    let inventory = &game.sessions[&server_peer].inventory;
    assert_eq!(inventory.count_of(542), 250);
    assert_eq!(inventory.count_of(543), 250);
    assert_eq!(inventory.count_of(102), 250);
    assert_eq!(inventory.count_of(12600), 1);
    assert_eq!(inventory.count_of(9640), 1);
    assert_eq!(inventory.count_of(818), 1);
    assert_eq!(inventory.count_of(98), 1);
    send(&mut client, peer, "action|refresh_item_data\n");
    let messages = pump(&mut game, &mut client, |_, messages| messages.iter().any(|message| matches!(message, Message::GamePacket { tank, .. } if tank.kind == TYPE_SEND_ITEM_DATABASE_DATA)));
    let database = messages.into_iter().find_map(|message| match message {
        Message::GamePacket { tank, extra } if tank.kind == TYPE_SEND_ITEM_DATABASE_DATA => Some(extra),
        _ => None,
    }).unwrap();
    let database = ItemsDat::parse(database).unwrap();
    assert_eq!(database.hash(), game.items.as_ref().unwrap().hash());
    assert!(database.get(60004).unwrap().available);
    send(&mut client, peer, "action|store\n");
    let messages = pump(&mut game, &mut client, |_, messages| messages.iter().any(|message| match message {
        Message::GamePacket { extra, .. } => extra.windows(b"Mythical Infinity Rayman's Fist".len()).any(|part| part == b"Mythical Infinity Rayman's Fist"),
        _ => false,
    }));
    assert!(!messages.is_empty());
    game.sessions.get_mut(&server_peer).unwrap().inventory.remove(542, 10);
    game.checkpoint().unwrap();
    client.disconnect(peer, 0);
    pump(&mut game, &mut client, |game, _| game.sessions.is_empty());
    drop(client);
    let mut client = Host::client("127.0.0.1:0".parse().unwrap(), 1, 2).unwrap();
    let peer = client.connect(address, 2, 0).unwrap();
    pump(&mut game, &mut client, |_, messages| messages.iter().any(|message| matches!(message, Message::ServerHello)));
    send(&mut client, peer, "tankIDName|NetworkTest\ntankIDPass|network-test-password\n");
    pump(&mut game, &mut client, |game, _| game.sessions.values().any(|session| session.user_id == account.id));
    assert_eq!(game.sessions.values().next().unwrap().inventory.count_of(542), 240);
}
