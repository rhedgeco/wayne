use std::os::fd::OwnedFd;

use crate::{Buffer, parser::Builder};

use super::{uint, utils};

pub struct Parser {
    bytes: Option<Builder<utils::VecParser>>,
    len: uint::Parser,
    padding: u32,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            bytes: None,
            len: uint::Parser::new(),
            padding: 0,
        }
    }
}

impl crate::Parser for Parser {
    type Output = Box<[u8]>;

    fn parse(
        &mut self,
        mut bytes: impl Buffer<u8>,
        mut fds: impl Buffer<OwnedFd>,
    ) -> Option<Self::Output> {
        // try to get any pending bytes builder
        let mut builder = match self.bytes.take() {
            Some(bytes) => bytes,
            None => {
                // if there was none, build the length and padding
                let len = self.len.parse(&mut bytes, &mut fds)?;
                self.padding = ((len + 3) & !3) - len;
                Builder::new(utils::VecParser::new(len as usize))
            }
        };

        // then try to parse all the array bytes
        if builder.parse(&mut bytes, &mut fds).is_none() {
            self.bytes = Some(builder);
            return None;
        };

        // keep parsing bytes until the padding is zero
        while self.padding > 0 {
            if bytes.take().is_none() {
                self.bytes = Some(builder);
                return None;
            }

            self.padding -= 1;
        }

        // then consume and return the bytes
        Some(builder.finish()?.into_boxed_slice())
    }
}
