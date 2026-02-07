use bytemuck::cast_slice;

use crate::types::{Fixed, NewId, ObjectId, RawString};

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
pub struct Message<'a> {
    pub id: ObjectId,
    pub opcode: u16,
    pub data: &'a [u32],
}

impl<'a> Message<'a> {
    /// Constructs a [`MessageParser`] out of the contained `data` slice
    pub fn parser(&self) -> MessageParser<'a> {
        MessageParser { data: self.data }
    }
}

/// A parser for decoding arguments out of a [`Message`] data payload
pub struct MessageParser<'a> {
    data: &'a [u32],
}

impl<'a> MessageParser<'a> {
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

        // convert the u32 slice to a u8 slice
        // the array size counts the number of bytes in the array
        let all_bytes: &[u8] = cast_slice(&self.data[1..]);

        // ensure there are enough bytes left in the slice
        if all_bytes.len() < array_size {
            return None;
        }

        // then subslice the array to get the exact contents out
        let array_bytes = &all_bytes[0..array_size];

        // ceiling divide the byte count by 4
        // this finds how many u32 values were used
        // also add 1 to the value, since the count iself was used
        // then swap out the data slice with the shortened one
        let consume_count = array_size.div_ceil(4) + 1;
        self.data = &self.data[consume_count..];

        // finally, return the calculated array bytes
        Some(array_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn mock_message<'a>(data: &'a [u32]) -> Message<'a> {
        Message {
            id: ObjectId(0),
            opcode: 0,
            data,
        }
    }

    #[test]
    fn parse_uint() {
        const VALUE: u32 = 120;
        let mut parser = mock_message(&[VALUE]).parser();
        let value = parser.parse_uint().unwrap();
        assert_eq!(value, VALUE);
    }

    #[test]
    fn parse_int() {
        const VALUE: i32 = -120;
        let mut parser = mock_message(&[VALUE as u32]).parser();
        let value = parser.parse_int().unwrap();
        assert_eq!(value, VALUE);
    }

    #[test]
    fn parse_fixed() {
        const VALUE: i32 = -120;
        let mut parser = mock_message(&[VALUE as u32]).parser();
        let value = parser.parse_fixed().unwrap();
        assert_eq!(value.0, VALUE);
    }

    #[test]
    fn parse_object_id() {
        const VALUE: u32 = 120;
        let mut parser = mock_message(&[VALUE]).parser();
        let value = parser.parse_object_id().unwrap();
        assert_eq!(value.0, VALUE);
    }

    #[test]
    fn parse_new_id() {
        const VALUE: u32 = 120;
        let mut parser = mock_message(&[VALUE]).parser();
        let value = parser.parse_new_id().unwrap();
        assert_eq!(value.0, VALUE);
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

        let mut parser = mock_message(DATA).parser();
        let array = parser.parse_array().unwrap();
        assert_eq!(array, ARRAY);
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

        let mut parser = mock_message(DATA).parser();
        let string = parser.parse_string().unwrap();
        assert_eq!(string.0, ARRAY);
    }
}
