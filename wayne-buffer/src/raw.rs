use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

pub trait RawRef {
    fn len(&self) -> usize;
    fn as_ptr(&self) -> *const u8;
}

pub trait RawMut: RawRef {
    fn as_mut_ptr(&mut self) -> *mut u8;
}

impl RawRef for [u8] {
    fn len(&self) -> usize {
        Self::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        Self::as_ptr(self)
    }
}

impl RawMut for [u8] {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        Self::as_mut_ptr(self)
    }
}

impl RawRef for [MaybeUninit<u8>] {
    fn len(&self) -> usize {
        Self::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        Self::as_ptr(self).cast()
    }
}

impl RawMut for [MaybeUninit<u8>] {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        Self::as_mut_ptr(self).cast()
    }
}

impl<R: RawRef, T: Deref<Target = R>> RawRef for T {
    fn len(&self) -> usize {
        R::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        R::as_ptr(self)
    }
}

impl<R: RawMut, T: DerefMut<Target = R>> RawMut for T {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        R::as_mut_ptr(self)
    }
}
