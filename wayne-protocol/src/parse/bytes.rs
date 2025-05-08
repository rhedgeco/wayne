use std::{mem::MaybeUninit, os::unix::prelude::OwnedFd};

use crate::{
    Buffer, Parser,
    parser::{ParseError, ParseResult},
};

pub struct Many {
    vec: Option<Vec<u8>>,
    len: usize,
}

impl Many {
    pub const fn new(len: usize) -> Many {
        Many { vec: None, len }
    }
}

impl Parser for Many {
    type Output = Box<[u8]>;

    fn parse(&mut self, mut bytes: impl Buffer<u8>, _: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        let mut vec = self
            .vec
            .take()
            .unwrap_or_else(|| Vec::with_capacity(self.len));

        loop {
            if vec.len() == self.len {
                return Ok(vec.into_boxed_slice());
            }

            let Some(byte) = bytes.take() else {
                self.vec = Some(vec);
                return Err(ParseError::Incomplete);
            };

            vec.push(byte);
        }
    }
}

pub struct Sized<const LEN: usize> {
    bytes: [MaybeUninit<u8>; LEN],
    index: usize,
}

impl<const LEN: usize> Sized<LEN> {
    pub const fn new() -> Self {
        Self {
            bytes: [MaybeUninit::uninit(); LEN],
            index: 0,
        }
    }
}

impl<const LEN: usize> Parser for Sized<LEN> {
    type Output = [u8; LEN];

    fn parse(&mut self, mut bytes: impl Buffer<u8>, _: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        loop {
            if self.index == LEN {
                self.index = 0;
                let ptr = self.bytes.as_ptr();
                return Ok(unsafe { core::ptr::read(ptr.cast()) });
            }

            let Some(byte) = bytes.take() else {
                return Err(ParseError::Incomplete);
            };

            self.bytes[self.index].write(byte);
            self.index += 1;
        }
    }
}
