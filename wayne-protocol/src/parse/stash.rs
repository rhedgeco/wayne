use std::os::unix::prelude::OwnedFd;

use crate::{Buffer, Parser, parser::ParseResult};

pub struct Stash<P: Parser> {
    stash: Option<P::Output>,
    parser: P,
}

impl<P: Parser> Stash<P> {
    pub const fn new(parser: P) -> Self {
        Self {
            stash: None,
            parser,
        }
    }

    pub fn take(&mut self) -> P::Output {
        self.stash
            .take()
            .expect("stash must be parsed before calling take")
    }
}

impl<P: Parser> Parser for Stash<P> {
    type Output = ();

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        if self.stash.is_some() {
            return Ok(());
        }

        self.stash = Some(self.parser.parse(bytes, fds)?);
        Ok(())
    }
}
