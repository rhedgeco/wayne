use std::os::fd::OwnedFd;

use crate::{Buffer, types::RawId};

use super::uint;

pub struct Parser {
    value: uint::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            value: uint::Parser::new(),
        }
    }
}

impl crate::Parser for Parser {
    type Output = RawId;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
        let value = self.value.parse(bytes, fds)?;
        Some(RawId::from_value(value))
    }
}
