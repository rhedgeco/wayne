use std::os::fd::OwnedFd;

use crate::types::{Fixed, RawId};

pub trait Buffer {
    fn take_byte(&mut self) -> Option<u8>;
    fn take_fd(&mut self) -> Option<OwnedFd>;
}

impl<T: Buffer> Buffer for &mut T {
    fn take_byte(&mut self) -> Option<u8> {
        T::take_byte(self)
    }

    fn take_fd(&mut self) -> Option<OwnedFd> {
        T::take_fd(self)
    }
}

enum BuildState<P: Parser> {
    Output(P::Output),
    Parser(P),
}

pub struct Builder<P: Parser, F> {
    state: BuildState<P>,
    fds: Vec<OwnedFd>,
    map: F,
}

impl<P, F, O> Builder<P, F>
where
    P: Parser,
    F: FnOnce(P::Output, Box<[OwnedFd]>) -> O,
{
    pub fn build(mut self, mut buffer: impl Buffer) -> Result<O, Self> {
        let out = match self.state {
            BuildState::Output(out) => out,
            // keep taking bytes until parsing is complete
            BuildState::Parser(mut parser) => loop {
                // if we run out of bytes, just return incomplete
                let Some(byte) = buffer.take_byte() else {
                    self.state = BuildState::Parser(parser);
                    return Err(self);
                };

                // otherwise parse the next byte
                match parser.parse(byte) {
                    Err(p) => parser = p,
                    Ok(out) => break out,
                }
            },
        };

        // keep taking file descriptors until complete
        while self.fds.len() < self.fds.capacity() {
            // if we run out of fds, just return incomplete
            let Some(fd) = buffer.take_fd() else {
                self.state = BuildState::Output(out);
                return Err(self);
            };

            // othwerwise store the received file descriptor
            self.fds.push(fd);
        }

        // if we are finished parsing, and we have all file descriptors, then the builder is complete
        Ok((self.map)(out, self.fds.into_boxed_slice()))
    }
}

impl<P: Parser> BuilderExt for P {}
pub trait BuilderExt: Parser {
    fn builder<F>(self, fds: usize, f: F) -> Builder<Self, F>
    where
        F: FnOnce(Self::Output, Box<[OwnedFd]>),
    {
        Builder {
            state: BuildState::Parser(self),
            fds: Vec::with_capacity(fds),
            map: f,
        }
    }
}

#[allow(unused_variables)]
pub trait Parser: Sized {
    type Output;
    fn parse(self, byte: u8) -> Result<Self::Output, Self>;
}

pub fn sink(count: usize) -> Sink {
    Sink { count }
}

pub fn bytes(len: usize) -> Bytes {
    Bytes {
        bytes: Vec::with_capacity(len),
    }
}

pub fn i32() -> impl Parser<Output = i32> {
    bytes(4).map(|b| i32::from_ne_bytes([b[0], b[1], b[2], b[4]]))
}

pub fn u32() -> impl Parser<Output = u32> {
    bytes(4).map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[4]]))
}

pub fn fixed() -> impl Parser<Output = Fixed> {
    i32().map(|value| Fixed::from_raw(value))
}

pub fn raw_id() -> impl Parser<Output = RawId> {
    u32().map(|value| RawId::from_value(value))
}

pub fn array() -> impl Parser<Output = Box<[u8]>> {
    // https://wayland.freedesktop.org/docs/html/ch04.html#sect-Protocol-Wire-Format
    // array - Starts with 32-bit array size in bytes, followed by the array contents verbatim, and finally padding to a 32-bit boundary.

    // parse the 32 bits representing the array size
    u32()
        .then(|len| {
            // parse the array bytes
            bytes(len as usize)
        })
        .then(|bytes| {
            // find the remaining padding required
            let padded_len = (bytes.len() + 3) & !3;
            let remaining = padded_len - bytes.len();

            // the consume the padding and return the original array
            sink(remaining).map(|_| bytes)
        })
}

pub fn string() -> impl Parser<Output = String> {
    // parse an array, and then interpret the bytes as a string
    array().map(|bytes| String::from_utf8_lossy(&bytes).to_string())
}

pub struct Sink {
    count: usize,
}

impl Parser for Sink {
    type Output = usize;
    fn parse(mut self, _: u8) -> Result<Self::Output, Self> {
        self.count -= 1;
        if self.count == 0 {
            return Ok(self.count);
        }

        Err(self)
    }
}

pub struct Bytes {
    bytes: Vec<u8>,
}

impl Parser for Bytes {
    type Output = Box<[u8]>;

    fn parse(mut self, byte: u8) -> Result<Self::Output, Self> {
        self.bytes.push(byte);
        if self.bytes.len() == self.bytes.capacity() {
            return Ok(self.bytes.into_boxed_slice());
        }

        Err(self)
    }
}

pub struct Map<P, F> {
    parser: P,
    map: F,
}

impl<P, F, O> Parser for Map<P, F>
where
    P: Parser,
    F: FnOnce(P::Output) -> O,
{
    type Output = O;

    fn parse(mut self, byte: u8) -> Result<Self::Output, Self> {
        match self.parser.parse(byte) {
            Ok(out) => Ok((self.map)(out)),
            Err(parser) => {
                self.parser = parser;
                Err(self)
            }
        }
    }
}

impl<P: Parser> MapExt for P {}
pub trait MapExt: Parser {
    fn map<F, O>(self, f: F) -> Map<Self, F>
    where
        F: FnOnce(Self::Output) -> O,
    {
        Map {
            parser: self,
            map: f,
        }
    }
}

enum ThenState<P1, P2, F> {
    First(Map<P1, F>),
    Second(P2),
}

pub struct Then<P1, P2, F> {
    state: ThenState<P1, P2, F>,
}

impl<P1, P2, F> Parser for Then<P1, P2, F>
where
    P1: Parser,
    P2: Parser,
    F: FnOnce(P1::Output) -> P2,
{
    type Output = P2::Output;

    fn parse(mut self, byte: u8) -> Result<Self::Output, Self> {
        self.state = match self.state {
            ThenState::First(first) => match first.parse(byte) {
                Ok(second) => ThenState::Second(second),
                Err(first) => ThenState::First(first),
            },
            ThenState::Second(second) => match second.parse(byte) {
                Err(second) => ThenState::Second(second),
                Ok(out) => return Ok(out),
            },
        };

        Err(self)
    }
}

impl<P: Parser> ThenExt for P {}
pub trait ThenExt: Parser {
    fn then<P2, F>(self, f: F) -> Then<Self, P2, F>
    where
        P2: Parser,
        F: FnOnce(Self::Output) -> P2,
    {
        Then {
            state: ThenState::First(self.map(f)),
        }
    }
}
