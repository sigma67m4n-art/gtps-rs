use crate::cursor::Writer;

pub const TYPE_FLOAT: u8 = 1;
pub const TYPE_STRING: u8 = 2;
pub const TYPE_VEC2: u8 = 3;
pub const TYPE_VEC3: u8 = 4;
pub const TYPE_UINT32: u8 = 5;
pub const TYPE_INT32: u8 = 9;

#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    Float(f32),
    String(String),
    Vec2(f32, f32),
    Vec3(f32, f32, f32),
    Uint(u32),
    Int(i32),
}

impl Variant {
    #[must_use]
    pub fn type_id(&self) -> u8 {
        match self {
            Self::Float(_) => TYPE_FLOAT,
            Self::String(_) => TYPE_STRING,
            Self::Vec2(_, _) => TYPE_VEC2,
            Self::Vec3(_, _, _) => TYPE_VEC3,
            Self::Uint(_) => TYPE_UINT32,
            Self::Int(_) => TYPE_INT32,
        }
    }

    pub fn write(&self, writer: &mut Writer) {
        match self {
            Self::Float(value) => {
                writer.f32(*value);
            }
            Self::String(value) => {
                writer.str32(value);
            }
            Self::Vec2(x, y) => {
                writer.f32(*x).f32(*y);
            }
            Self::Vec3(x, y, z) => {
                writer.f32(*x).f32(*y).f32(*z);
            }
            Self::Uint(value) => {
                writer.u32(*value);
            }
            Self::Int(value) => {
                writer.i32(*value);
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VariantList {
    name: String,
    args: Vec<Variant>,
}

impl VariantList {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, value: Variant) -> Self {
        self.args.push(value);
        self
    }

    pub fn push(&mut self, value: Variant) -> &mut Self {
        self.args.push(value);
        self
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn args(&self) -> &[Variant] {
        &self.args
    }

    #[must_use]
    pub fn variant_count(&self) -> u8 {
        (self.args.len() + 1).min(u8::MAX as usize) as u8
    }

    pub fn encode_into(&self, writer: &mut Writer) {
        writer.u8(self.variant_count()).u8(0).u8(TYPE_STRING);
        writer.str32(&self.name);

        for (index, arg) in self.args.iter().enumerate() {
            writer.u8((index + 1) as u8).u8(arg.type_id());
            arg.write(writer);
        }
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer::with_capacity(1 + (self.args.len() + 1) * 8);
        self.encode_into(&mut writer);
        writer.into_vec()
    }
}
