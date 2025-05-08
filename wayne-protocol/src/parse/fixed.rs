use std::os::unix::prelude::OwnedFd;

use fixed::types::I24F8;

use crate::{Buffer, parser::ParseResult};

use super::int;

pub struct Parser {
    bits: int::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self { bits: int() }
    }
}

impl crate::Parser for Parser {
    type Output = f32;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        let bits = self.bits.parse(bytes, fds)?;
        Ok(I24F8::from_bits(bits).to_num())
    }
}
