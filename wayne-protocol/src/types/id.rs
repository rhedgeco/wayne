use std::marker::PhantomData;

use derivative::Derivative;
use derive_more::Display;

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
