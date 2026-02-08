use std::{os::fd::OwnedFd, process::Output};

use bytemuck::cast_slice;

use crate::types::{Fixed, NewId, ObjectId, RawString};

/// A wayland protocol message
pub trait Message {
    /// The builder for the protocol message
    type Builder: Builder<Output = Self>;
}

/// A builder trait for constructing [`Message`] types.
///
/// Parsing happens in two stages since the file descriptors get passed separately from other args.
/// - `parse` - constructs the initial builder with the data from the message.
/// - `complete` - provides the builder with access to the store for file descriptors.
pub trait Builder: Sized {
    type Output: Message<Builder = Self>;
    fn parse(data: DataParser) -> Option<Self>;
    fn complete(self, fds: impl Iterator<Item = OwnedFd>) -> Option<Output>;
}

/// An untyped wayland message.
///
/// According to the [wayland wire format](https://wayland.freedesktop.org/docs/html/ch04.html#sect-Protocol-Wire-Format),
/// a message contains an object id, an opcode, and the raw bytes of the data payload.
///
/// The payload describes the request/event arguments. Every argument is always aligned to 32-bits.
/// Where padding is required, the value of padding bytes is undefined.
/// There is no prefix that describes the type, but it is inferred implicitly from the xml specification.
/// The payload contents can be deduced by referencing the interface associated with the object id,
/// and determining the event/request type based on the opcode.
pub struct RawMessage<'a> {
    pub id: ObjectId,
    pub opcode: u16,
    pub data: &'a [u32],
}

impl<'a> RawMessage<'a> {
    /// Parses message data into the protocol message builder
    pub fn parse<T: Message>(&self) -> Option<T::Builder> {
        T::Builder::parse(DataParser { data: self.data })
    }
}

/// A parser for args in a [`Message`] data payload
pub struct DataParser<'a> {
    data: &'a [u32],
}

impl<'a> DataParser<'a> {
    /// Parses a [`u32`] from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_uint(&mut self) -> Option<u32> {
        // get the first value out of the data slice
        let value = *self.data.get(0)?;

        // then chop off the first value in the slice
        self.data = &self.data[1..];

        // finally, return the calculated value
        Some(value)
    }

    /// Parses a [`i32`] from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_int(&mut self) -> Option<i32> {
        // parse a normal uint, and just cast it to an i32
        self.parse_uint().map(|v| v as i32)
    }

    /// Parses a [`Fixed`] from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_fixed(&mut self) -> Option<Fixed> {
        // parse a normal int, and just wrap it in a Fixed
        self.parse_int().map(Fixed)
    }

    /// Parses a [`ObjectId`] from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_object_id(&mut self) -> Option<ObjectId> {
        // parse a normal uint, and just wrap it in a ObjectId
        self.parse_uint().map(ObjectId)
    }

    /// Parses a [`NewId`] from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_new_id(&mut self) -> Option<NewId> {
        // parse a normal uint, and just wrap it in a NewId
        self.parse_uint().map(NewId)
    }

    /// Parses a [`RawString`] from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_string(&mut self) -> Option<RawString<'a>> {
        // a string can be simply constructed by first parsing a byte array
        // then converting those bytes into the correct string
        self.parse_array().map(RawString)
    }

    /// Parses a [`u8`] slice from the data payload.
    ///
    /// Returns `None` if there were not enough bytes.
    pub fn parse_array(&mut self) -> Option<&'a [u8]> {
        // first extract the byte count
        let array_size = *self.data.get(0)? as usize;

        // convert the u32 slice to a u8 slice with possible array contents
        // we use u8 here since the array_size value represents the byte count
        let all_bytes: &[u8] = cast_slice(&self.data[1..]);

        // then subslice the array to get the exact contents out
        let array_bytes = all_bytes.get(..array_size)?;

        // ceiling divide the byte count by 4
        // this finds how many u32 values are used by the array (including padding)
        // also add 1 to the value, since the count itself was used
        let consume_count = array_size.div_ceil(4) + 1;

        // then swap out the data slice with the shortened one
        self.data = &self.data[consume_count..];

        // finally, return the calculated array bytes
        Some(array_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn parser<'a>(data: &'a [u32]) -> DataParser<'a> {
        DataParser { data }
    }

    #[test]
    fn parse_uint() {
        const VALUE: u32 = 120;
        let mut parser = parser(&[VALUE]);
        let value = parser.parse_uint().unwrap();
        assert_eq!(value, VALUE);

        assert!(parser.parse_int().is_none());
    }

    #[test]
    fn parse_int() {
        const VALUE: i32 = -120;
        let mut parser = parser(&[VALUE as u32]);
        let value = parser.parse_int().unwrap();
        assert_eq!(value, VALUE);

        assert!(parser.parse_int().is_none());
    }

    #[test]
    fn parse_fixed() {
        const VALUE: i32 = -120;
        let mut parser = parser(&[VALUE as u32]);
        let value = parser.parse_fixed().unwrap();
        assert_eq!(value.0, VALUE);

        assert!(parser.parse_int().is_none());
    }

    #[test]
    fn parse_object_id() {
        const VALUE: u32 = 120;
        let mut parser = parser(&[VALUE]);
        let value = parser.parse_object_id().unwrap();
        assert_eq!(value.0, VALUE);

        assert!(parser.parse_int().is_none());
    }

    #[test]
    fn parse_new_id() {
        const VALUE: u32 = 120;
        let mut parser = parser(&[VALUE]);
        let value = parser.parse_new_id().unwrap();
        assert_eq!(value.0, VALUE);

        assert!(parser.parse_int().is_none());
    }

    #[test]
    fn parse_array() {
        // arrays should be padded to 32 bit boundaries
        const ARRAY: &[u8] = &[1, 2, 3, 4, 5];
        const DATA: &[u32] = &[
            5, // first u32 in the array is the length
            u32::from_ne_bytes([1, 2, 3, 4]),
            u32::from_ne_bytes([5, 0, 0, 0]),
        ];

        let mut parser = parser(DATA);
        let array = parser.parse_array().unwrap();
        assert_eq!(array, ARRAY);

        assert!(parser.parse_int().is_none());
    }

    #[test]
    fn fail_parse_array() {
        const DATA: &[u32] = &[
            // set the length to 5
            5,
            // but only include 4 bytes
            u32::from_ne_bytes([1, 2, 3, 4]),
        ];

        let mut parser = parser(DATA);
        assert!(parser.parse_array().is_none());
    }

    #[test]
    fn parse_string() {
        // strings are identical to arrays, just wrapped
        const ARRAY: &[u8] = &[5, 4, 3, 2, 1];
        const DATA: &[u32] = &[
            5, // first u32 in the array is the length
            u32::from_ne_bytes([5, 4, 3, 2]),
            u32::from_ne_bytes([1, 0, 0, 0]),
        ];

        let mut parser = parser(DATA);
        let string = parser.parse_string().unwrap();
        assert_eq!(string.0, ARRAY);

        assert!(parser.parse_int().is_none());
    }
}
