use std::mem::MaybeUninit;

pub trait Buffer: private::SealedBuffer {}
impl<T: private::SealedBuffer> Buffer for T {}
pub(crate) mod private {
    pub trait SealedBuffer {
        unsafe fn assume_init(&self, len: usize) -> &[u8];
        fn buffer_ptr(&mut self) -> *mut u8;
        fn buffer_len(&self) -> usize;
    }
}

impl private::SealedBuffer for &mut [u8] {
    unsafe fn assume_init(&self, len: usize) -> &[u8] {
        &self[0..len]
    }

    fn buffer_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    fn buffer_len(&self) -> usize {
        self.len()
    }
}

impl private::SealedBuffer for &mut [MaybeUninit<u8>] {
    unsafe fn assume_init(&self, len: usize) -> &[u8] {
        unsafe { core::mem::transmute(&self[0..len]) }
    }

    fn buffer_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr() as _
    }

    fn buffer_len(&self) -> usize {
        self.len()
    }
}
