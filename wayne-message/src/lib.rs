/// A raw wayland message
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message<'a> {
    pub object_id: u32,
    pub opcode: u16,
    pub body: &'a [u8],
}

/// A buffer that parses and produces [`RawMessage`] data
pub struct MessageBuffer<Buf> {
    parse_start: usize,
    parse_end: usize,
    buffer: Buf,
}

impl<Buf: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer<Buf> {
    /// Returns a new message buffer backed by the provided `buffer`
    pub fn new(buffer: Buf) -> Self {
        Self {
            parse_start: 0,
            parse_end: 0,
            buffer,
        }
    }

    /// Manually mark the next `count` bytes as filled
    ///
    /// This usually means the free_space buffer was filled by copying bytes directly
    pub fn mark_filled(&mut self, count: usize) {
        let buffer_len = self.buffer.as_ref().len();
        self.parse_end = self.parse_end.saturating_add(count).min(buffer_len);
    }

    /// Write as many `bytes` into the buffer as possible
    ///
    /// Returns the number of bytes successfully written
    pub fn write(&mut self, bytes: &[u8]) -> usize {
        let free_space = self.free_space();
        let count = free_space.len().min(bytes.len());
        free_space[..count].copy_from_slice(&bytes[..count]);
        self.parse_end += count;
        count
    }

    /// Returns the free buffer space
    ///
    /// This can be used to fill the buffer with data before advancing the parser
    pub fn free_space(&mut self) -> &mut [u8] {
        let buffer = self.buffer.as_mut();

        // if there is parsed data at the front
        if self.parse_start > 0 {
            // copy the remaining bytes to the start
            buffer.copy_within(self.parse_start..self.parse_end, 0);

            // and update the parse indices
            self.parse_end -= self.parse_start;
            self.parse_start = 0;
        }

        // then return the free space subslice
        &mut buffer[self.parse_end..]
    }

    /// Parses and returns the next [`RawMessage`] in the buffer.
    ///
    /// Returns `None` if no more complete messages could be parsed.
    pub fn parse(&mut self) -> Option<Message> {
        // get the initialized buffer and remove the already parsed bytes
        let data = &self.buffer.as_ref()[self.parse_start..self.parse_end];

        // SAFETY:
        // MessageBuffer invariants require that all data up to init_len is properly initialized
        let data: &[u8] = unsafe { core::mem::transmute(data) };

        // ensure we have enough data to parse the header
        if data.len() < 8 {
            return None;
        }

        // parse the second word in the header to get the length
        let second_word = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]);

        // extract the message length and ensure that it is at least 8 bytes
        let message_len = ((second_word >> 16) as u16).max(8);

        // pad message length to align to multiple of 4 (32 bits)
        let message_len = ((message_len + 3) & !3) as usize;

        // ensure there is enough data for the rest of the message
        if data.len() < message_len {
            return None;
        }

        // increment the parse index for the next iteration
        self.parse_start += message_len;

        // build and return the parsed message
        Some(Message {
            object_id: u32::from_ne_bytes([data[0], data[1], data[2], data[3]]),
            opcode: (second_word & 0xFFFF) as u16,
            body: &data[8..message_len],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJID: u32 = 420;
    const OPCODE: u16 = 69;
    const LENGTH: u16 = (8 + BODY.len()) as u16;
    const BODY: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8]; // must be multiple of 32 bits length
    const WORD2: u32 = (OPCODE as u32) | ((LENGTH as u32) << 16);
    const BYTES: &[u8] = constcat::concat_bytes!(&OBJID.to_ne_bytes(), &WORD2.to_ne_bytes(), BODY);

    #[test]
    fn copy_message() {
        // build the buffer
        let mut buffer = MessageBuffer::new([0; BYTES.len()]);

        // copy the message into the free space
        buffer.free_space().copy_from_slice(BYTES);
        buffer.mark_filled(BYTES.len());

        // parse and validate the message
        let message = buffer.parse().unwrap();
        assert_eq!(message.object_id, OBJID);
        assert_eq!(message.opcode, OPCODE);
        assert_eq!(message.body, BODY);
        assert!(buffer.parse().is_none());
    }

    #[test]
    fn write_message() {
        // build the buffer
        let mut buffer = MessageBuffer::new([0; BYTES.len()]);

        // write the message into the free space
        let write_len = buffer.write(BYTES);
        assert_eq!(write_len, BYTES.len());

        // parse and validate the message
        let message = buffer.parse().unwrap();
        assert_eq!(message.object_id, OBJID);
        assert_eq!(message.opcode, OPCODE);
        assert_eq!(message.body, BODY);
        assert!(buffer.parse().is_none());
    }

    #[test]
    fn partial_parse() {
        // build the buffer
        const PARTIAL: usize = BYTES.len() / 2;
        const LEN: usize = BYTES.len() + PARTIAL;
        let mut buffer = MessageBuffer::new([0; LEN]);

        // write one message and a half messages into the free space
        let write_len = buffer.write(BYTES);
        assert_eq!(write_len, BYTES.len());
        let write_len = buffer.write(BYTES);
        assert_eq!(write_len, PARTIAL);

        // parse the first message
        let message = buffer.parse().unwrap();
        assert_eq!(message.object_id, OBJID);
        assert_eq!(message.opcode, OPCODE);
        assert_eq!(message.body, BODY);
        assert!(buffer.parse().is_none());

        // write the rest of the second message
        let write_len = buffer.write(&BYTES[PARTIAL..]);
        assert_eq!(write_len, BYTES.len() - PARTIAL);

        // parse the second message
        let message = buffer.parse().unwrap();
        assert_eq!(message.object_id, OBJID);
        assert_eq!(message.opcode, OPCODE);
        assert_eq!(message.body, BODY);
        assert!(buffer.parse().is_none());
    }
}
