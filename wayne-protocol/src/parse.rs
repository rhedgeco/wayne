use std::os::fd::OwnedFd;

use fixed::types::I24F8;

use crate::{
    buffer::Buffer,
    types::{NewId, ObjId, RawId},
};

pub fn u16() -> impl Parser<Output = u16> {
    Bytes::new(2).map(|b| u16::from_ne_bytes([b[0], b[1]]))
}

pub fn i32() -> impl Parser<Output = i32> {
    Bytes::new(4).map(|b| i32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn u32() -> impl Parser<Output = u32> {
    Bytes::new(4).map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn f32() -> impl Parser<Output = f32> {
    i32().map(|value| I24F8::from_bits(value).to_num())
}

pub fn raw_id() -> impl Parser<Output = RawId> {
    u32().map(|value| RawId::from_value(value))
}

pub fn obj_id<T>() -> impl Parser<Output = ObjId<T>> {
    raw_id().map(|id| id.to_obj())
}

pub fn new_id<T>() -> impl Parser<Output = NewId<T>> {
    raw_id().map(|id| id.to_new())
}

pub fn array() -> impl Parser<Output = Box<[u8]>> {
    // https://wayland.freedesktop.org/docs/html/ch04.html#sect-Protocol-Wire-Format
    // array - Starts with 32-bit array size in bytes, followed by the array contents verbatim, and finally padding to a 32-bit boundary.

    // parse the 32 bits representing the array size
    u32()
        .then(|len| {
            // parse the array bytes
            Bytes::new(len as usize)
        })
        .then(|bytes| {
            // find the remaining padding required
            let padded_len = (bytes.len() + 3) & !3;
            let remaining = padded_len - bytes.len();

            // the consume the padding and return the original array
            Consume::new(remaining, 0).map(|_| bytes)
        })
}

pub fn string() -> impl Parser<Output = String> {
    // parse an array, and then interpret the bytes as a string
    array().map(|bytes| String::from_utf8_lossy(&bytes).to_string())
}

pub fn fd() -> impl Parser<Output = OwnedFd> {
    Fd::new()
}

pub enum ParseError<P> {
    Incomplete(P),
    Failed,
}

impl<P> ParseError<P> {
    pub fn map<P2>(self, f: impl FnOnce(P) -> P2) -> ParseError<P2> {
        match self {
            ParseError::Incomplete(parser) => ParseError::Incomplete((f)(parser)),
            ParseError::Failed => ParseError::Failed,
        }
    }
}

pub type ParseResult<P> = Result<<P as Parser>::Output, ParseError<P>>;

pub trait Parser: Sized {
    type Output;
    fn parse(
        self,
        bytes: &mut impl Buffer<u8>,
        fds: &mut impl Buffer<OwnedFd>,
    ) -> ParseResult<Self>;
}

pub struct Fd(());

impl Fd {
    pub fn new() -> Self {
        Self(())
    }
}

impl Parser for Fd {
    type Output = OwnedFd;

    fn parse(self, _: &mut impl Buffer<u8>, fds: &mut impl Buffer<OwnedFd>) -> ParseResult<Self> {
        match fds.take() {
            None => Err(ParseError::Incomplete(self)),
            Some(fd) => Ok(fd),
        }
    }
}

pub struct Pass<T> {
    item: T,
}

impl<T> Pass<T> {
    pub fn new(item: T) -> Self {
        Self { item }
    }
}

impl<T> Parser for Pass<T> {
    type Output = T;

    fn parse(self, _: &mut impl Buffer<u8>, _: &mut impl Buffer<OwnedFd>) -> ParseResult<Self> {
        Ok(self.item)
    }
}

pub struct Consume {
    bytes: usize,
    fds: usize,
}

impl Consume {
    pub fn new(bytes: usize, fds: usize) -> Self {
        Self { bytes, fds }
    }
}

impl Parser for Consume {
    type Output = (usize, usize);
    fn parse(
        mut self,
        bytes: &mut impl Buffer<u8>,
        fds: &mut impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        // consume bytes until complete
        while self.bytes > 0 {
            if bytes.take().is_none() {
                return Err(ParseError::Incomplete(self));
            };

            self.bytes -= 1;
        }

        // consume fds until complete
        while self.fds > 0 {
            if fds.take().is_none() {
                return Err(ParseError::Incomplete(self));
            };

            self.fds -= 1;
        }

        Ok((self.bytes, self.fds))
    }
}

pub struct Bytes {
    bytes: Vec<u8>,
}

impl Bytes {
    pub fn new(size: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(size),
        }
    }
}

impl Parser for Bytes {
    type Output = Box<[u8]>;
    fn parse(
        mut self,
        bytes: &mut impl Buffer<u8>,
        _: &mut impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        // keep taking bytes until the complete
        while self.bytes.len() < self.bytes.capacity() {
            // if bytes are exhausted, return incomplete
            let Some(byte) = bytes.take() else {
                return Err(ParseError::Incomplete(self));
            };

            // otherwise append the bytes to the vec
            self.bytes.push(byte);
        }

        // if all bytes were captured, return the bytes
        Ok(self.bytes.into_boxed_slice())
    }
}

pub struct Map<P, F> {
    parser: P,
    map: F,
}

impl<P, F, Out> Parser for Map<P, F>
where
    P: Parser,
    F: FnOnce(P::Output) -> Out,
{
    type Output = Out;

    fn parse(
        self,
        bytes: &mut impl Buffer<u8>,
        fds: &mut impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        match self.parser.parse(bytes, fds) {
            Ok(out) => Ok((self.map)(out)),
            Err(err) => Err(err.map(|parser| Self {
                map: self.map,
                parser,
            })),
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

    fn parse(
        mut self,
        bytes: &mut impl Buffer<u8>,
        fds: &mut impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        self.state = match self.state {
            ThenState::First(first) => match first.parse(bytes, fds) {
                Ok(second) => ThenState::Second(second),
                Err(err) => match err {
                    ParseError::Incomplete(first) => ThenState::First(first),
                    ParseError::Failed => return Err(ParseError::Failed),
                },
            },
            ThenState::Second(second) => match second.parse(bytes, fds) {
                Ok(out) => return Ok(out),
                Err(err) => match err {
                    ParseError::Incomplete(second) => ThenState::Second(second),
                    ParseError::Failed => return Err(ParseError::Failed),
                },
            },
        };

        Err(ParseError::Incomplete(self))
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

pub struct Some<P> {
    parser: P,
}

impl<T, P> Parser for Some<P>
where
    P: Parser<Output = Option<T>>,
{
    type Output = T;

    fn parse(
        self,
        bytes: &mut impl Buffer<u8>,
        fds: &mut impl Buffer<OwnedFd>,
    ) -> ParseResult<Self> {
        match self.parser.parse(bytes, fds) {
            Ok(Some(out)) => Ok(out),
            Ok(None) => Err(ParseError::Failed),
            Err(err) => Err(err.map(|parser| Self { parser })),
        }
    }
}

impl<T, P: Parser<Output = Option<T>>> OptionExt for P {}
pub trait OptionExt: Parser {
    fn some<T>(self) -> Some<Self>
    where
        Self: Parser<Output = Option<T>>,
    {
        Some { parser: self }
    }
}
