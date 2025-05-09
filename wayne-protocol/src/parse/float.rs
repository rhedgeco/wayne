use std::os::fd::OwnedFd;

use fixed::types::I28F4;

use crate::Buffer;

use super::int;

pub struct Parser {
    bits: int::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            bits: int::Parser::new(),
        }
    }
}

impl crate::Parser for Parser {
    type Output = f32;

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
        let bits = self.bits.parse(bytes, fds)?;
        Some(I28F4::from_bits(bits).to_num())
    }
}
