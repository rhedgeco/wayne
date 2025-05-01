use std::marker::PhantomData;

use fixed::types::I24F8;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed(I24F8);

impl Into<f32> for Fixed {
    fn into(self) -> f32 {
        self.0.to_num()
    }
}

impl From<f32> for Fixed {
    fn from(value: f32) -> Self {
        Self(I24F8::from_num(value))
    }
}

impl Into<f64> for Fixed {
    fn into(self) -> f64 {
        self.0.to_num()
    }
}

impl From<f64> for Fixed {
    fn from(value: f64) -> Self {
        Self(I24F8::from_num(value))
    }
}

impl Fixed {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId<T> {
    _type: PhantomData<fn() -> T>,
    raw: u32,
}

impl<T> ObjectId<T> {
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            _type: PhantomData,
            raw,
        }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewId<T> {
    _type: PhantomData<fn() -> T>,
    raw: u32,
}

impl<T> NewId<T> {
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            _type: PhantomData,
            raw,
        }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }
}
