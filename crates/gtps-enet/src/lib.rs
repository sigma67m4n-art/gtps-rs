mod host;
mod packet;
mod sys;

pub use host::{Event, Host, Role};
pub use packet::Packet;

use core::fmt;
use core::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeerId(pub usize);

impl PeerId {
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Disconnected,
    Connecting,
    AcknowledgingConnect,
    ConnectionPending,
    ConnectionSucceeded,
    Connected,
    DisconnectLater,
    Disconnecting,
    AcknowledgingDisconnect,
    Zombie,
    Unknown(i32),
}

impl PeerState {
    #[must_use]
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Disconnected,
            1 => Self::Connecting,
            2 => Self::AcknowledgingConnect,
            3 => Self::ConnectionPending,
            4 => Self::ConnectionSucceeded,
            5 => Self::Connected,
            6 => Self::DisconnectLater,
            7 => Self::Disconnecting,
            8 => Self::AcknowledgingDisconnect,
            9 => Self::Zombie,
            other => Self::Unknown(other),
        }
    }

    #[must_use]
    pub fn is_connected(self) -> bool {
        self == Self::Connected
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: &'static str,
}

impl Error {
    #[must_use]
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }

    #[must_use]
    pub fn message(&self) -> &'static str {
        self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for Error {}

static INIT: Once = Once::new();
static INIT_RESULT: AtomicI32 = AtomicI32::new(-1);

pub fn initialize() -> Result<(), Error> {
    INIT.call_once(|| {
        let rc = unsafe { sys::enet_initialize() };
        INIT_RESULT.store(rc, Ordering::SeqCst);
    });

    if INIT_RESULT.load(Ordering::SeqCst) == 0 {
        Ok(())
    } else {
        Err(Error::new("enet_initialize failed"))
    }
}

pub fn shutdown() {
    unsafe { sys::enet_deinitialize() };
}
