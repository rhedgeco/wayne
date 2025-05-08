use std::os::unix::prelude::OwnedFd;

use crate::{Buffer, parser::ParseResult};

use super::array;

pub struct Parser {
    array: array::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self { array: array() }
    }
}

impl crate::Parser for Parser {
    type Output = String;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        let array = self.array.parse(bytes, fds)?;
        Ok(String::from_utf8_lossy(&array).to_string())
    }
}
