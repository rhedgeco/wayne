use std::mem::MaybeUninit;

/// A raw wayland message
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message<'a> {
    pub object_id: u32,
    pub opcode: u16,
    pub body: &'a [u8],
}

/// A buffer that parses and produces [`RawMessage`] data
pub struct MessageBuffer<'a> {
    buffer: &'a mut [MaybeUninit<u8>],
    parse_index: usize,
    init_len: usize,
}

impl<'a> MessageBuffer<'a> {
    /// Returns a new message buffer backed by the provided `buffer`
    pub fn new(buffer: &'a mut [MaybeUninit<u8>]) -> Self {
        Self {
            buffer,
            parse_index: 0,
            init_len: 0,
        }
    }

    /// Attempts to write as many bytes from `data` into the buffer as possible,
    /// returning the number of bytes that were successfully written.
    pub fn write_bytes(&mut self, data: &[u8]) -> usize {
        let uninit = self.get_uninit_space();
        let src = data.as_ptr();
        let dst = uninit.as_mut_ptr().cast::<u8>();
        let count = uninit.len().min(data.len());

        // SAFETY:
        // src and dst are valid for length count.
        // count was built by taking the lowest of the two lengths.
        // both slices come from safe rust code and are properly aligned
        unsafe { core::ptr::copy(src, dst, count) };
        self.init_len += count;
        count
    }

    /// Marks the next `count` bytes in the buffer as initialized and ready for parsing
    ///
    /// # SAFETY
    /// Behavior is undefined if any of the following conditions are violated:
    /// - `count` must be less than or equal to the length of the uninitialized buffer space
    /// - the first `count` bytes of the uninitialized buffer must have been properly initialized
    ///
    /// You may initialize the buffer by calling [`get_uninit_space`](Self::get_uninit_space) and writing bytes to it
    pub unsafe fn set_init(&mut self, count: usize) {
        self.init_len += count
    }

    /// Returns the a mutable subslice of the buffer that is currently uninitialized
    pub fn get_uninit_space(&mut self) -> &mut [MaybeUninit<u8>] {
        // if there are messages that have already been parsed
        if self.parse_index > 0 {
            // assert that the parse index is less than the total init length
            debug_assert!(self.parse_index < self.init_len);

            // copy the remaining buffer contents to the beginning
            self.buffer.copy_within(self.parse_index..self.init_len, 0);

            // reset the init length to reflect shifted values
            self.init_len -= self.parse_index;
        }

        // return the subslice of the buffer that is yet to be initialized
        &mut self.buffer[self.init_len..]
    }

    /// Parses and returns the next [`RawMessage`] in the buffer.
    ///
    /// Returns `None` if no more complete messages could be parsed.
    pub fn parse(&mut self) -> Option<Message> {
        // get the initialized buffer and remove the already parsed bytes
        let data = &self.buffer[self.parse_index..self.init_len];

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
        self.parse_index += message_len;

        // build and return the parsed message
        Some(Message {
            object_id: u32::from_ne_bytes([data[0], data[1], data[2], data[3]]),
            opcode: (second_word & 0xFFFF) as u16,
            body: &data[8..message_len],
        })
    }
}
