use std::os::fd::OwnedFd;

use crate::{Buffer, parser::Builder, types::id::CustomNewId};

use super::{string, uint};

pub struct Parser {
    name: Builder<string::Parser>,
    version: Builder<uint::Parser>,
    value: Builder<uint::Parser>,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            name: Builder::new(string::Parser::new()),
            version: Builder::new(uint::Parser::new()),
            value: Builder::new(uint::Parser::new()),
        }
    }
}

impl crate::Parser for Parser {
    type Output = CustomNewId;

    fn parse(
        &mut self,
        mut bytes: impl Buffer<u8>,
        mut fds: impl Buffer<OwnedFd>,
    ) -> Option<Self::Output> {
        self.name.parse(&mut bytes, &mut fds)?;
        self.version.parse(&mut bytes, &mut fds)?;
        self.value.parse(&mut bytes, &mut fds)?;

        Some(CustomNewId {
            name: self.name.finish()?,
            version: self.version.finish()?,
            value: self.value.finish()?,
        })
    }
}
