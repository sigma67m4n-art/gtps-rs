use crate::cursor::{Reader, Result, Writer};
use crate::tank::{TankPacket, TYPE_CALL_FUNCTION};
use crate::variant::VariantList;

pub const MSG_SERVER_HELLO: u32 = 1;
pub const MSG_GENERIC_TEXT: u32 = 2;
pub const MSG_GAME_MESSAGE: u32 = 3;
pub const MSG_GAME_PACKET: u32 = 4;
pub const MSG_ERROR: u32 = 5;
pub const MSG_TRACK: u32 = 6;
pub const MSG_LOG_REQUEST: u32 = 7;
pub const MSG_LOG_RESPONSE: u32 = 8;

pub const NET_ID_BROADCAST: i32 = -1;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    ServerHello,
    GenericText(String),
    GameMessage(String),
    Error(String),
    Track(String),
    GamePacket { tank: TankPacket, extra: Vec<u8> },
    Other { kind: u32, body: Vec<u8> },
}

fn framed(kind: u32, body: &[u8]) -> Vec<u8> {
    let mut writer = Writer::with_capacity(4 + body.len());
    writer.u32(kind).bytes(body);
    writer.into_vec()
}

impl Message {
    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(data);
        let kind = reader.u32()?;
        let body = reader.rest();

        Ok(match kind {
            MSG_SERVER_HELLO => Self::ServerHello,
            MSG_GENERIC_TEXT => Self::GenericText(String::from_utf8_lossy(body).into_owned()),
            MSG_GAME_MESSAGE => Self::GameMessage(String::from_utf8_lossy(body).into_owned()),
            MSG_ERROR => Self::Error(String::from_utf8_lossy(body).into_owned()),
            MSG_TRACK => Self::Track(String::from_utf8_lossy(body).into_owned()),
            MSG_GAME_PACKET => {
                let mut payload = Reader::new(body);
                let tank = TankPacket::decode(&mut payload)?;
                let extra = if tank.is_extended() {
                    payload.rest().to_vec()
                } else {
                    Vec::new()
                };
                Self::GamePacket { tank, extra }
            }
            other => Self::Other {
                kind: other,
                body: body.to_vec(),
            },
        })
    }

    #[must_use]
    pub fn kind(&self) -> u32 {
        match self {
            Self::ServerHello => MSG_SERVER_HELLO,
            Self::GenericText(_) => MSG_GENERIC_TEXT,
            Self::GameMessage(_) => MSG_GAME_MESSAGE,
            Self::Error(_) => MSG_ERROR,
            Self::Track(_) => MSG_TRACK,
            Self::GamePacket { .. } => MSG_GAME_PACKET,
            Self::Other { kind, .. } => *kind,
        }
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::ServerHello => server_hello(),
            Self::GenericText(text) => generic_text(text),
            Self::GameMessage(text) => game_message(text),
            Self::Error(text) => framed(MSG_ERROR, text.as_bytes()),
            Self::Track(text) => framed(MSG_TRACK, text.as_bytes()),
            Self::GamePacket { tank, extra } => game_packet(tank, extra),
            Self::Other { kind, body } => framed(*kind, body),
        }
    }

    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::GenericText(text) | Self::GameMessage(text) | Self::Error(text) => Some(text),
            Self::Track(text) => Some(text),
            _ => None,
        }
    }
}

#[must_use]
pub fn server_hello() -> Vec<u8> {
    framed(MSG_SERVER_HELLO, &[])
}

#[must_use]
pub fn generic_text(text: &str) -> Vec<u8> {
    framed(MSG_GENERIC_TEXT, text.as_bytes())
}

#[must_use]
pub fn game_message(text: &str) -> Vec<u8> {
    framed(MSG_GAME_MESSAGE, text.as_bytes())
}

#[must_use]
pub fn game_packet(tank: &TankPacket, extra: &[u8]) -> Vec<u8> {
    let mut payload = Writer::with_capacity(TankPacket::SIZE + extra.len());
    tank.encode(&mut payload);
    payload.bytes(extra);
    framed(MSG_GAME_PACKET, payload.as_slice())
}

#[must_use]
pub fn variant_packet(list: &VariantList, net_id: i32, delay: i32) -> Vec<u8> {
    let extra = list.encode();
    let mut tank = TankPacket::new(TYPE_CALL_FUNCTION);
    tank.net_id = net_id;
    tank.int_data = delay;
    tank.set_extended(extra.len());
    game_packet(&tank, &extra)
}

#[cfg(test)]
mod tests {
    use super::{game_packet, variant_packet, Message, MSG_GAME_PACKET};
    use crate::tank::{TankPacket, TYPE_SEND_MAP_DATA};
    use crate::variant::{Variant, VariantList};

    #[test]
    fn tank_frame_round_trips() {
        let mut tank = TankPacket::new(TYPE_SEND_MAP_DATA);
        tank.net_id = 7;
        tank.set_extended(3);

        let frame = game_packet(&tank, &[1, 2, 3]);
        assert_eq!(u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]), MSG_GAME_PACKET);

        match Message::decode(&frame).expect("decode") {
            Message::GamePacket { tank, extra } => {
                assert_eq!(tank.kind, TYPE_SEND_MAP_DATA);
                assert_eq!(tank.net_id, 7);
                assert_eq!(extra, vec![1, 2, 3]);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn variant_frame_round_trips() {
        let list = VariantList::new("OnConsoleMessage").arg(Variant::String("hi".into()));
        let frame = variant_packet(&list, -1, 0);

        match Message::decode(&frame).expect("decode") {
            Message::GamePacket { tank, extra } => {
                assert!(tank.is_extended());
                assert_eq!(tank.data_size as usize, extra.len());
                assert_eq!(extra[0], 2);
                assert_eq!(extra[1], 0);
                assert_eq!(extra[2], 2);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}
