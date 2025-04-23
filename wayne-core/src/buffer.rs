use std::mem::MaybeUninit;

pub trait Buffer: sealed::Buffer {}
impl<T: sealed::Buffer> Buffer for T {}

impl sealed::Buffer for [u8] {
    fn len(&self) -> usize {
        <[_]>::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        <[_]>::as_ptr(self)
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        <[_]>::as_mut_ptr(self)
    }
}

impl<const SIZE: usize> sealed::Buffer for [u8; SIZE] {
    fn len(&self) -> usize {
        <[_]>::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        <[_]>::as_ptr(self)
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        <[_]>::as_mut_ptr(self)
    }
}

impl sealed::Buffer for [MaybeUninit<u8>] {
    fn len(&self) -> usize {
        <[_]>::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        <[_]>::as_ptr(self) as _
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        <[_]>::as_mut_ptr(self) as _
    }
}

impl<const SIZE: usize> sealed::Buffer for [MaybeUninit<u8>; SIZE] {
    fn len(&self) -> usize {
        <[_]>::len(self)
    }

    fn as_ptr(&self) -> *const u8 {
        <[_]>::as_ptr(self) as _
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        <[_]>::as_mut_ptr(self) as _
    }
}

mod sealed {
    pub trait Buffer {
        fn len(&self) -> usize;
        fn as_ptr(&self) -> *const u8;
        fn as_mut_ptr(&mut self) -> *mut u8;
    }
}
