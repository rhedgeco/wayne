use std::os::fd::OwnedFd;

use crate::{Buffer, types::RawString};

use super::array;

pub struct Parser {
    array: array::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            array: array::Parser::new(),
        }
    }
}

impl crate::Parser for Parser {
    type Output = RawString;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
        let array = self.array.parse(bytes, fds)?;
        Some(RawString::from_bytes(array))
    }
}
