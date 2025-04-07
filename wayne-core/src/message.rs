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
///
/// When parsing, a message may be incomplete,
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

    /// Builds a message stream that resumes parsing messages
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
                    let object_id = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    let opcode = u16::from_ne_bytes([bytes[4], bytes[5]]);
                    let size = u16::from_ne_bytes([bytes[6], bytes[7]]).max(8) as usize;
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
