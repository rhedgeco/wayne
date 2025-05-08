use std::os::fd::OwnedFd;

use thiserror::Error;

use crate::Buffer;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("parsing is incomplete")]
    Incomplete,
    #[error("parsing failed")]
    Failed,
}

pub type ParseResult<P> = Result<<P as Parser>::Output, ParseError>;

pub trait Parser {
    type Output;
    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> ParseResult<Self>;
}
