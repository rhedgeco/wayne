use std::marker::PhantomData;

use derivative::Derivative;
use derive_more::Display;
use fixed::types::I24F8;

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed(I24F8);

impl Into<f32> for Fixed {
    fn into(self) -> f32 {
        self.to_f32()
    }
}

impl From<f32> for Fixed {
    fn from(value: f32) -> Self {
        Self::from_f32(value)
    }
}

impl Into<f64> for Fixed {
    fn into(self) -> f64 {
        self.to_f64()
    }
}

impl From<f64> for Fixed {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl Fixed {
    pub fn to_f32(self) -> f32 {
        self.0.to_num()
    }

    pub fn to_f64(self) -> f64 {
        self.0.to_num()
    }

    pub fn from_f32(value: f32) -> Self {
        Self(I24F8::from_num(value))
    }

    pub fn from_f64(value: f64) -> Self {
        Self(I24F8::from_num(value))
    }

    pub const fn from_be_bytes(bytes: [u8; 4]) -> Self {
        Self(I24F8::from_be_bytes(bytes))
    }

    pub const fn from_le_bytes(bytes: [u8; 4]) -> Self {
        Self(I24F8::from_le_bytes(bytes))
    }

    pub const fn from_ne_bytes(bytes: [u8; 4]) -> Self {
        Self(I24F8::from_ne_bytes(bytes))
    }
}

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawId(u32);

impl RawId {
    pub const fn value(self) -> u32 {
        self.0
    }

    pub const fn from_value(value: u32) -> Self {
        Self(value)
    }

    pub const fn to_obj<T>(self) -> ObjId<T> {
        ObjId::from_raw(self)
    }

    pub const fn to_new<T>(self) -> NewId<T> {
        NewId::from_raw(self)
    }
}

#[repr(transparent)]
#[derive(Debug, Derivative, Display)]
#[derivative(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{raw}")]
pub struct ObjId<T> {
    _type: PhantomData<fn() -> T>,
    raw: RawId,
}

impl<T> From<RawId> for ObjId<T> {
    fn from(raw: RawId) -> Self {
        Self::from_raw(raw)
    }
}

impl<T> ObjId<T> {
    pub const fn raw(self) -> RawId {
        self.raw
    }

    pub const fn from_raw(raw: RawId) -> Self {
        Self {
            _type: PhantomData,
            raw,
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Derivative, Display)]
#[derivative(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{raw}")]
pub struct NewId<T> {
    _type: PhantomData<fn() -> T>,
    raw: RawId,
}

impl<T> From<RawId> for NewId<T> {
    fn from(raw: RawId) -> Self {
        Self::from_raw(raw)
    }
}

impl<T> NewId<T> {
    pub const fn raw(self) -> RawId {
        self.raw
    }

    pub const fn from_raw(raw: RawId) -> Self {
        Self {
            _type: PhantomData,
            raw,
        }
    }
}
