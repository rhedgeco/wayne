use std::os::fd::OwnedFd;

use crate::types::{Fixed, NewId, ObjectId, RawString};

/// A parser interface for extracting wayland arguments
pub trait Parser<'a> {
    fn parse_int(&mut self) -> Option<i32>;
    fn parse_uint(&mut self) -> Option<u32>;
    fn parse_fixed(&mut self) -> Option<Fixed>;
    fn parse_string(&mut self) -> Option<RawString<'a>>;
    fn parse_object(&mut self) -> Option<ObjectId>;
    fn parse_newid(&mut self) -> Option<NewId>;
    fn parse_array(&mut self) -> Option<&'a [u8]>;
    fn parse_fd(&mut self) -> Option<OwnedFd>;
}
