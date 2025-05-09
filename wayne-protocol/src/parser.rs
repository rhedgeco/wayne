use std::os::fd::OwnedFd;

use crate::Buffer;

pub trait Parser {
    type Output;
    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output>;
}

pub struct Builder<P: Parser> {
    output: Option<P::Output>,
    parser: P,
}

impl<P: Parser> Builder<P> {
    pub const fn new(parser: P) -> Self {
        Self {
            output: None,
            parser,
        }
    }

    pub const fn finish(&mut self) -> Option<P::Output> {
        self.output.take()
    }
}

impl<P: Parser> Parser for Builder<P> {
    type Output = ();

    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
        if self.output.is_some() {
            return Some(());
        }

        let output = self.parser.parse(bytes, fds)?;
        self.output = Some(output);
        Some(())
    }
}
