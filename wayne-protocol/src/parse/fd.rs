use std::os::fd::OwnedFd;

use crate::Buffer;

pub struct Parser(());

impl Parser {
    pub const fn new() -> Self {
        Self(())
    }
}

impl crate::Parser for Parser {
    type Output = OwnedFd;

    fn parse(&mut self, _: impl Buffer<u8>, mut fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
        fds.take()
    }
}
