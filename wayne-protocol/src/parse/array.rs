use std::os::unix::prelude::OwnedFd;

use crate::{
    Buffer,
    parser::{ParseError, ParseResult},
};

use super::{bytes, uint};

pub struct Parser {
    many: Option<bytes::Many>,
    len: uint::Parser,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            many: None,
            len: uint(),
        }
    }
}

impl crate::Parser for Parser {
    type Output = Box<[u8]>;

    fn parse(
        &mut self,
        mut bytes: impl Buffer<u8>,
        mut fds: impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        let mut many = match self.many.take() {
            Some(many) => many,
            None => {
                let len = self.len.parse(&mut bytes, &mut fds)?;
                let padded_len = (len + 3) & !3;
                bytes::Many::new(padded_len as usize)
            }
        };

        match many.parse(bytes, fds) {
            Ok(out) => Ok(out),
            Err(ParseError::Failed) => Err(ParseError::Failed),
            Err(ParseError::Incomplete) => {
                self.many = Some(many);
                Err(ParseError::Incomplete)
            }
        }
    }
}
