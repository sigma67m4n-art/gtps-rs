use gtps_proto::{TankPacket, Text};

use crate::game::session::Session;

#[must_use]
pub fn join_request(text: &Text) -> Option<String> {
    if text.get("action")? != "join_request" {
        return None;
    }

    let name = text.get("name")?.trim().to_uppercase();
    (!name.is_empty()).then_some(name)
}

#[must_use]
pub fn spawn_string(session: &Session, local: bool) -> String {
    let tile_x = TankPacket::pixel_to_tile(session.pos_x);
    let tile_y = TankPacket::pixel_to_tile(session.pos_y);

    let mut out = String::new();
    out.push_str("spawn|avatar\n");
    out.push_str(&format!("netID|{}\n", session.net_id));
    out.push_str(&format!("userID|{}\n", session.user_id));
    out.push_str("colrect|0|0|20|30\n");
    out.push_str(&format!("posXY|{tile_x}|{tile_y}\n"));
    out.push_str(&format!("name|`w{}``\n", session.name));
    out.push_str(&format!("country|{}\n", session.country));
    out.push_str("invis|0\n");
    out.push_str("mstate|0\n");
    out.push_str("smstate|0\n");
    out.push_str("onlineID|\n");

    if local {
        out.push_str("\ntype|local\n");
    }

    out
}

#[must_use]
pub fn remove_string(net_id: i32, user_id: u32) -> String {
    format!("netID|{net_id}\npId|{user_id}\n")
}

#[must_use]
pub fn spawn_position(width: usize, door_row: usize) -> (f32, f32) {
    let x = (width as f32 / 2.0) * 32.0 + 16.0;
    let y = (door_row as f32 - 1.0) * 32.0 + 16.0;
    (x, y)
}
