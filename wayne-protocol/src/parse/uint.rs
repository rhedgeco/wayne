use std::os::unix::prelude::OwnedFd;

use crate::{Buffer, parser::ParseResult};

use super::bytes;

pub struct Parser {
    bytes: bytes::Sized<4>,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            bytes: bytes::Sized::new(),
        }
    }
}

impl crate::Parser for Parser {
    type Output = u32;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        let bytes = self.bytes.parse(bytes, fds)?;
        Ok(u32::from_ne_bytes(bytes))
    }
}
