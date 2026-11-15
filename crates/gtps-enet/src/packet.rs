use core::ffi::c_void;
use core::fmt;
use core::slice;

use crate::sys;

pub struct Packet {
    raw: *mut sys::ENetPacket,
}

impl Packet {
    #[must_use]
    pub fn reliable(data: impl AsRef<[u8]>) -> Self {
        Self::with_flags(data.as_ref(), sys::PACKET_FLAG_RELIABLE)
    }

    #[must_use]
    pub fn unreliable(data: impl AsRef<[u8]>) -> Self {
        Self::with_flags(data.as_ref(), 0)
    }

    fn with_flags(data: &[u8], flags: u32) -> Self {
        let raw =
            unsafe { sys::enet_packet_create(data.as_ptr().cast::<c_void>(), data.len(), flags) };
        Self { raw }
    }

    pub(crate) fn from_raw(raw: *mut sys::ENetPacket) -> Self {
        Self { raw }
    }

    pub(crate) fn into_raw(self) -> *mut sys::ENetPacket {
        let raw = self.raw;
        core::mem::forget(self);
        raw
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        unsafe {
            let ptr = sys::gtps_enet_packet_data(self.raw);
            let len = sys::gtps_enet_packet_size(self.raw);
            if ptr.is_null() || len == 0 {
                &[]
            } else {
                slice::from_raw_parts(ptr, len)
            }
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Packet").field("len", &self.len()).finish()
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sys::gtps_enet_packet_destroy(self.raw) };
        }
    }
}
