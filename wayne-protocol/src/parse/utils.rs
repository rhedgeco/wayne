use std::{mem::MaybeUninit, os::fd::OwnedFd};

use crate::Buffer;

pub struct VecParser {
    vec: Option<Vec<u8>>,
    len: usize,
}

impl VecParser {
    pub const fn new(len: usize) -> Self {
        Self { vec: None, len }
    }
}

impl crate::Parser for VecParser {
    type Output = Vec<u8>;

    fn parse(
        &mut self,
        mut bytes: impl Buffer<u8>,
        _: impl Buffer<OwnedFd>,
    ) -> Option<Self::Output> {
        let mut vec = self
            .vec
            .take()
            .unwrap_or_else(|| Vec::with_capacity(self.len));

        while vec.len() < self.len {
            let Some(byte) = bytes.take() else {
                self.vec = Some(vec);
                return None;
            };

            vec.push(byte);
        }

        Some(vec)
    }
}

pub struct ArrayParser<const LEN: usize> {
    bytes: [MaybeUninit<u8>; LEN],
    index: usize,
}

impl<const LEN: usize> ArrayParser<LEN> {
    pub const fn new() -> Self {
        Self {
            bytes: [MaybeUninit::uninit(); LEN],
            index: 0,
        }
    }
}

impl<const LEN: usize> crate::Parser for ArrayParser<LEN> {
    type Output = [u8; LEN];

    fn parse(
        &mut self,
        mut bytes: impl Buffer<u8>,
        _: impl Buffer<OwnedFd>,
    ) -> Option<Self::Output> {
        while self.index < LEN {
            let byte = bytes.take()?;
            self.bytes[self.index].write(byte);
            self.index += 1;
        }

        Some(unsafe { core::ptr::read(self.bytes.as_ptr().cast()) })
    }
}
