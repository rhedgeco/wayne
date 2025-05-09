use std::{any::type_name, fmt::Debug, marker::PhantomData};

use derivative::Derivative;
use derive_more::Display;

use super::RawString;

#[repr(transparent)]
#[derive(Derivative, Display)]
#[derivative(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{value}")]
pub struct NewId<T> {
    _type: PhantomData<fn() -> T>,
    value: u32,
}

impl<T> From<u32> for NewId<T> {
    fn from(value: u32) -> Self {
        Self::from_value(value)
    }
}

impl<T> NewId<T> {
    pub const fn value(self) -> u32 {
        self.value
    }

    pub const fn from_value(value: u32) -> Self {
        Self {
            _type: PhantomData,
            value,
        }
    }
}

impl<T> Debug for NewId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("NewId<{}>", type_name::<T>()))
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{value}({name}:{version})")]
pub struct CustomNewId {
    pub name: RawString,
    pub version: u32,
    pub value: u32,
}

#[repr(transparent)]
#[derive(Derivative, Display)]
#[derivative(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{value}")]
pub struct ObjectId<T> {
    _type: PhantomData<fn() -> T>,
    value: u32,
}

impl<T> From<u32> for ObjectId<T> {
    fn from(value: u32) -> Self {
        Self::from_value(value)
    }
}

impl<T> ObjectId<T> {
    pub const fn value(self) -> u32 {
        self.value
    }

    pub const fn from_value(raw: u32) -> Self {
        Self {
            _type: PhantomData,
            value: raw,
        }
    }
}

impl<T> Debug for ObjectId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("ObjId<{}>", type_name::<T>()))
            .field("value", &self.value)
            .finish()
    }
}
