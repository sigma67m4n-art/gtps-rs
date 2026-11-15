use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoError {
    message: &'static str,
}

impl ProtoError {
    #[must_use]
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }

    #[must_use]
    pub fn message(&self) -> &'static str {
        self.message
    }
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for ProtoError {}

pub type Result<T> = core::result::Result<T, ProtoError>;

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| ProtoError::new("length overflow"))?;

        if end > self.data.len() {
            return Err(ProtoError::new("unexpected end of buffer"));
        }

        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    pub fn u64(&mut self) -> Result<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        self.take(count)
    }

    pub fn skip(&mut self, count: usize) -> Result<()> {
        self.take(count).map(|_| ())
    }

    pub fn str16(&mut self) -> Result<&'a str> {
        let len = self.u16()? as usize;
        core::str::from_utf8(self.take(len)?).map_err(|_| ProtoError::new("invalid utf8"))
    }

    pub fn str32(&mut self) -> Result<&'a str> {
        let len = self.u32()? as usize;
        core::str::from_utf8(self.take(len)?).map_err(|_| ProtoError::new("invalid utf8"))
    }

    pub fn rest(&mut self) -> &'a [u8] {
        let out = &self.data[self.pos..];
        self.pos = self.data.len();
        out
    }
}

#[derive(Debug, Default, Clone)]
pub struct Writer {
    data: Vec<u8>,
}

impl Writer {
    #[must_use]
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn u8(&mut self, value: u8) -> &mut Self {
        self.data.push(value);
        self
    }

    pub fn i8(&mut self, value: i8) -> &mut Self {
        self.u8(value as u8)
    }

    pub fn u16(&mut self, value: u16) -> &mut Self {
        self.data.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn i16(&mut self, value: i16) -> &mut Self {
        self.u16(value as u16)
    }

    pub fn u32(&mut self, value: u32) -> &mut Self {
        self.data.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn i32(&mut self, value: i32) -> &mut Self {
        self.u32(value as u32)
    }

    pub fn u64(&mut self, value: u64) -> &mut Self {
        self.data.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn f32(&mut self, value: f32) -> &mut Self {
        self.u32(value.to_bits())
    }

    pub fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.data.extend_from_slice(value);
        self
    }

    pub fn zeros(&mut self, count: usize) -> &mut Self {
        self.data.resize(self.data.len() + count, 0);
        self
    }

    pub fn str16(&mut self, value: &str) -> &mut Self {
        let bytes = value.as_bytes();
        self.u16(bytes.len() as u16);
        self.bytes(bytes)
    }

    pub fn str32(&mut self, value: &str) -> &mut Self {
        let bytes = value.as_bytes();
        self.u32(bytes.len() as u32);
        self.bytes(bytes)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }
}
