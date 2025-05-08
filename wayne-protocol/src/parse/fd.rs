use std::os::fd::OwnedFd;

use crate::{
    Buffer,
    parser::{ParseError, ParseResult},
};

pub struct Parser(());

impl Parser {
    pub const fn new() -> Self {
        Self(())
    }
}

impl crate::Parser for Parser {
    type Output = OwnedFd;

    fn parse(&mut self, _: impl Buffer<u8>, mut fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        match fds.take() {
            Some(fd) => Ok(fd),
            None => Err(ParseError::Incomplete),
        }
    }
}
