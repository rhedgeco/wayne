#[derive(Debug, Clone)]
pub struct Message {
    pub object_id: u32,
    pub opcode: u16,
    pub body: Box<[u8]>,
}

#[derive(Debug)]
enum ParseState {
    Empty,
    IncompleteHeader(Vec<u8>),
    IncompleteBody {
        object_id: u32,
        opcode: u16,
        body: Vec<u8>,
    },
}

/// Parses messages byte by byte.
#[derive(Debug)]
pub struct MessageParser {
    state: ParseState,
}

impl MessageParser {
    /// Returns a new empty message parser
    pub fn new() -> Self {
        Self {
            state: ParseState::Empty,
        }
    }

    /// Builds a message stream that resumes decoding messages.
    ///
    /// Any unfinished messages will be stored and will continue parsing when more bytes are provided.
    pub fn parse<'a: 'b, 'b>(&'a mut self, bytes: &'b [u8]) -> MessageStream<'b> {
        MessageStream {
            state: &mut self.state,
            bytes,
        }
    }
}

pub struct MessageStream<'a> {
    state: &'a mut ParseState,
    bytes: &'a [u8],
}

impl<'a> Iterator for MessageStream<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // ensure the bytes are non-empty
            let bytes = self.bytes;
            if bytes.is_empty() {
                return None;
            }

            // then update the parse state
            match core::mem::replace(self.state, ParseState::Empty) {
                ParseState::Empty => {
                    *self.state = ParseState::IncompleteHeader(Vec::with_capacity(8))
                }
                ParseState::IncompleteHeader(mut header) => {
                    // get the cap for the header bytes
                    let cap = (8 - header.len()).min(bytes.len());

                    // extend the header with the necessary bytes
                    header.extend_from_slice(&bytes[0..cap]);
                    self.bytes = &bytes[cap..];

                    // if the header is incomplete,
                    // then the bytes are exhausted and we can return early.
                    if header.len() < 8 {
                        // put the state data back
                        *self.state = ParseState::IncompleteHeader(header);
                        return None;
                    }

                    // create the header values
                    let object_id =
                        u32::from_ne_bytes([header[0], header[1], header[2], header[3]]);
                    let opcode = u16::from_ne_bytes([header[4], header[5]]);
                    let size = u16::from_ne_bytes([header[6], header[7]]).max(8) as usize;
                    let size = (size + 3) & !3; //  round the size to the nearest 32 bit multiple
                    let body = Vec::with_capacity(size - 8);

                    // update the state with an incomplete body
                    *self.state = ParseState::IncompleteBody {
                        object_id,
                        opcode,
                        body,
                    };
                }
                ParseState::IncompleteBody {
                    object_id,
                    opcode,
                    mut body,
                } => {
                    // get the cap for the body bytes
                    let cap = (body.capacity() - body.len()).min(bytes.len());

                    // extend the body with the necessary bytes
                    body.extend_from_slice(&bytes[0..cap]);
                    self.bytes = &bytes[cap..];

                    // if the body is incomplete,
                    // then the bytes are exhausted and we can return early.
                    if body.len() < body.capacity() {
                        // put the state data back
                        *self.state = ParseState::IncompleteBody {
                            object_id,
                            opcode,
                            body,
                        };
                        return None;
                    }

                    // if the body is complete,
                    // build the message and return it.
                    return Some(Message {
                        object_id,
                        opcode,
                        body: body.into_boxed_slice(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use constcat::concat_bytes;

    use super::*;

    #[test]
    fn parse_small_chunks() {
        const OBJECT_ID: u32 = 10;
        const OPCODE: u16 = 12;
        const BODY: &[u8] = &[3, 4, 5, 6];
        const BYTES: &[u8] = concat_bytes!(
            &OBJECT_ID.to_ne_bytes(),
            &OPCODE.to_ne_bytes(),
            &12u16.to_ne_bytes(),
            BODY
        );

        // iterate in 4 byte chunks
        let mut message_count = 0;
        let mut parser = MessageParser::new();
        for bytes in BYTES.chunks(4) {
            for message in parser.parse(bytes) {
                message_count += 1;
                assert_eq!(&message.object_id, &OBJECT_ID);
                assert_eq!(&message.opcode, &OPCODE);
                assert_eq!(&*message.body, BODY);
            }
        }

        assert_eq!(message_count, 1);
    }

    #[test]
    fn parse_large_chunks() {
        const OBJECT_ID: u32 = 10;
        const OPCODE: u16 = 12;
        const BODY: &[u8] = &[3, 4, 5, 6];
        const BYTES: &[u8] = concat_bytes!(
            &OBJECT_ID.to_ne_bytes(),
            &OPCODE.to_ne_bytes(),
            &12u16.to_ne_bytes(),
            BODY
        );
        const MANY_BYTES: &[u8] = concat_bytes!(BYTES, BYTES, BYTES, BYTES, BYTES, BYTES);

        // iterate in 30 byte chunks
        let mut message_count = 0;
        let mut parser = MessageParser::new();
        for bytes in MANY_BYTES.chunks(30) {
            for message in parser.parse(bytes) {
                message_count += 1;
                assert_eq!(&message.object_id, &OBJECT_ID);
                assert_eq!(&message.opcode, &OPCODE);
                assert_eq!(&*message.body, BODY);
            }
        }

        assert_eq!(message_count, 6);
    }
}
