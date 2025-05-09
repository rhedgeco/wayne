use std::os::fd::OwnedFd;

use crate::Buffer;

use super::bytes;

pub struct Parser {
    bytes: bytes::SizedParser<4>,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            bytes: bytes::SizedParser::new(),
        }
    }
}

impl crate::Parser for Parser {
    type Output = u32;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
        let bytes = self.bytes.parse(bytes, fds)?;
        Some(u32::from_ne_bytes(bytes))
    }
}
