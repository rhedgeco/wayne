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

pub trait EnumValue: sealed::EnumValue {}
impl<T: sealed::EnumValue> EnumValue for T {}

#[repr(transparent)]
#[derive(Debug, Derivative)]
#[derivative(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawEnum<T, E> {
    _type: PhantomData<fn() -> E>,
    value: T,
}

impl<T, E> From<T> for RawEnum<T, E>
where
    T: EnumValue,
    E: TryFrom<u32>,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T, E> RawEnum<T, E>
where
    T: EnumValue,
    E: TryFrom<u32>,
{
    pub const fn new(value: T) -> Self {
        Self {
            _type: PhantomData,
            value,
        }
    }

    pub fn build(&self) -> Option<E> {
        let value = self.value.to_value()?;
        E::try_from(value).ok()
    }
}

mod sealed {
    pub trait EnumValue {
        fn to_value(&self) -> Option<u32>;
    }

    impl EnumValue for u32 {
        fn to_value(&self) -> Option<u32> {
            Some(*self)
        }
    }

    impl EnumValue for i32 {
        fn to_value(&self) -> Option<u32> {
            u32::try_from(*self).ok()
        }
    }
}
