use std::{ops::AddAssign, os::unix::prelude::OwnedFd};

use crate::{
    Buffer, Parser,
    parser::{ParseError, ParseResult},
};

struct CountBuf<'a, B> {
    count: &'a mut usize,
    buffer: B,
}

impl<T, B: Buffer<T>> Buffer<T> for CountBuf<'_, B> {
    fn take(&mut self) -> Option<T> {
        let item = self.buffer.take()?;
        self.count.add_assign(1);
        Some(item)
    }
}

pub struct Pad<P: Parser> {
    out: Option<P::Output>,
    count: usize,
    pad: usize,
    parser: P,
}

impl<P: Parser> Parser for Pad<P> {
    type Output = P::Output;

    fn parse(
        &mut self,
        mut bytes: impl Buffer<u8>,
        fds: impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        // try to take the output if its already complete
        let out = match self.out.take() {
            Some(out) => out,
            None => {
                // create a buffer to count bytes
                let buffer = CountBuf {
                    count: &mut self.count,
                    buffer: &mut bytes,
                };

                // parse the data using the counting buffer
                self.parser.parse(buffer, fds)?
            }
        };

        // take bytes until the padding is reached
        let padded = (self.count + self.pad) & !self.pad;
        while self.count < padded {
            if bytes.take().is_none() {
                return Err(ParseError::Incomplete);
            }

            self.count += 1;
        }

        // then return the output
        Ok(out)
    }
}

impl<P: Parser> PadExt for P {}
pub trait PadExt: Parser {
    fn pad(self, pad: usize) -> Pad<Self>
    where
        Self: Sized,
    {
        Pad {
            out: None,
            count: 0,
            pad: pad.saturating_sub(1),
            parser: self,
        }
    }
}
