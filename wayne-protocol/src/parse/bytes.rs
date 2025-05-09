use std::{mem::MaybeUninit, os::fd::OwnedFd};

use crate::Buffer;

pub struct Parser {
    vec: Option<Vec<u8>>,
    len: usize,
}

impl Parser {
    pub const fn new(len: usize) -> Self {
        Self { vec: None, len }
    }
}

impl crate::Parser for Parser {
    type Output = Box<[u8]>;

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

        Some(vec.into_boxed_slice())
    }
}

pub struct SizedParser<const LEN: usize> {
    bytes: [MaybeUninit<u8>; LEN],
    index: usize,
}

impl<const LEN: usize> SizedParser<LEN> {
    pub const fn new() -> Self {
        Self {
            bytes: [MaybeUninit::uninit(); LEN],
            index: 0,
        }
    }
}

impl<const LEN: usize> crate::Parser for SizedParser<LEN> {
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
