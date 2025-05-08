use std::os::unix::prelude::OwnedFd;

use crate::{Buffer, parser::ParseResult, types::RawId};

use super::uint;

pub struct Parser {
    value: uint::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self { value: uint() }
    }
}

impl crate::Parser for Parser {
    type Output = RawId;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        let value = self.value.parse(bytes, fds)?;
        Ok(RawId::from_value(value))
    }
}
