use std::mem::MaybeUninit;

use crate::raw::RawMut;

/// A buffer for storing and tracking parially initialized buffers
pub struct InitBuffer<Raw> {
    init_len: usize,
    raw: Raw,
}

impl<Raw: RawMut> InitBuffer<Raw> {
    /// Returns a new [`InitBuffer`] backed by `raw`
    pub fn new(raw: Raw) -> Self {
        Self { init_len: 0, raw }
    }

    /// Returns a reference to the initialized section of the buffer
    pub fn get_init(&self) -> &[u8] {
        let init_len = self.init_len;
        let init_ptr = self.raw.as_ptr();
        unsafe { core::slice::from_raw_parts(init_ptr, init_len) }
    }

    /// Returns a mutable reference to the initialized section of the buffer
    pub fn get_init_mut(&mut self) -> &mut [u8] {
        let init_len = self.init_len;
        let init_ptr = self.raw.as_mut_ptr();
        unsafe { core::slice::from_raw_parts_mut(init_ptr, init_len) }
    }

    /// Returns a reference to the uninitialized section of the buffer
    pub fn get_uninit(&self) -> &[MaybeUninit<u8>] {
        let uninit_len = self.raw.len() - self.init_len;
        let uninit_ptr = unsafe { self.raw.as_ptr().add(self.init_len).cast() };
        unsafe { core::slice::from_raw_parts(uninit_ptr, uninit_len) }
    }

    /// Returns a mutable reference to the uninitialized section of the buffer
    pub fn get_uninit_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        let uninit_len = self.raw.len() - self.init_len;
        let uninit_ptr = unsafe { self.raw.as_mut_ptr().add(self.init_len).cast() };
        unsafe { core::slice::from_raw_parts_mut(uninit_ptr, uninit_len) }
    }

    /// Writes as many `bytes` as possible to the uninitialized section of the internal buffer
    ///
    /// Returns the number of bytes that were successfully written
    pub fn write(&mut self, bytes: &[u8]) -> usize {
        // get the remaining uninitialized bytes
        let uninit = self.get_uninit_mut();

        // calculate the smaller length of the two buffers
        let count = bytes.len().min(uninit.len());

        // copy as many bytes as possible into the uninit buffer
        let src = bytes.as_ptr();
        let dst = uninit.as_mut_ptr().cast();
        unsafe { core::ptr::copy(src, dst, count) };

        // then return the number of copied bytes
        count
    }

    /// Consumes the first `count` bytes from the initialized section of the buffer
    ///
    /// If there is leftover initialized data, it will be moved to the front of the buffer
    pub fn consume(&mut self, count: usize) {
        if count >= self.init_len {
            // if the count is greater than or equal to the initialized length
            // the only thing we need to do is reset the init length to zero
            self.init_len = 0;
        } else {
            // if the count is less, we need to copy the remaining data to the front
            let dst = self.raw.as_mut_ptr();
            let src = unsafe { dst.add(count) };
            let count = self.init_len - count;
            unsafe { core::ptr::copy(src, dst, count) };

            // and then set the init_len to the remaining count
            self.init_len = count;
        }
    }

    /// Increments the initialized buffer length by `count`
    ///
    /// # Safety
    /// Behavior is undefined if any of the following conditions are violated:
    /// - the first `count` bytes in the uninitialized section of the buffer must be properly initialized
    ///
    /// You may initialize the buffer by calling [`get_uninit_mut`](Self::get_uninit_mut) and writing bytes to it
    ///
    /// # Panics
    /// Will panic if count is greater than the length of the uninitialized buffer section
    pub unsafe fn set_init(&mut self, count: usize) {
        let new_len = self.init_len + count;
        assert!(new_len < self.raw.len());
        self.init_len = new_len;
    }
}
