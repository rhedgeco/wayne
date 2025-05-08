use std::os::unix::prelude::OwnedFd;

use crate::{
    Buffer, Parser,
    parser::{ParseError, ParseResult},
};

pub struct Builder<P: Parser> {
    stash: Option<P::Output>,
    parser: P,
}

impl<P: Parser> Builder<P> {
    pub const fn new(parser: P) -> Self {
        Self {
            stash: None,
            parser,
        }
    }

    pub fn finish(&mut self) -> ParseResult<P> {
        self.stash.take().ok_or(ParseError::Failed)
    }
}

impl<P: Parser> Parser for Builder<P> {
    type Output = ();

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self> {
        if self.stash.is_some() {
            return Ok(());
        }

        self.stash = Some(self.parser.parse(bytes, fds)?);
        Ok(())
    }
}
