use std::{
    io, mem,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::net::UnixStream,
    },
};

use crate::Message;

/// A buffer that can be used to read wayland messages from a `UnixStream`
pub struct ReadBuffer<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    data_buf: Data,
    ctrl_buf: Ctrl,
    data_start: usize,
    ctrl_start: usize,
    data_end: usize,
    ctrl_end: Option<usize>,
}

impl<Data, Ctrl> Drop for ReadBuffer<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    fn drop(&mut self) {
        // ensure all file descriptors are parsed
        // this ensures none are left dangling
        while self.parse_fd().is_some() {}
    }
}

impl<Data, Ctrl> ReadBuffer<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    /// Returns a new read buffer backed by `data_buf` and `ctrl_buf`
    pub fn new(data_buf: Data, ctrl_buf: Ctrl) -> Self {
        Self {
            data_buf,
            ctrl_buf,
            data_start: 0,
            ctrl_start: 0,
            data_end: 0,
            ctrl_end: Some(0),
        }
    }

    /// Parse the next [`Message`] from the `Data` buffer
    ///
    /// Returns `None` if there are none left in the buffer
    pub fn parse_message(&mut self) -> Option<Message> {
        // get the section of the data buffer that has remaining message data in it
        let data = &self.data_buf.as_ref()[self.data_start..self.data_end];

        // ensure we have enough data to parse the header
        if data.len() < 8 {
            return None;
        }

        // parse the second word in the header to get the length
        let second_word = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]);

        // extract the message length and ensure that it is at least 8 bytes
        let message_len = ((second_word >> 16) as u16).max(8) as usize;

        // pad message length to align to multiple of 4 (32 bits)
        let padded_len = ((message_len + 3) & !3) as usize;

        // ensure there is enough data for the rest of the message
        if data.len() < message_len {
            return None;
        }

        // increment the data start index for the next iteration
        // and ensure the start index never jumps past the end index
        self.data_start = (self.data_start + padded_len).min(self.data_end);

        // build and return the parsed message
        Some(Message {
            object_id: u32::from_ne_bytes([data[0], data[1], data[2], data[3]]),
            opcode: (second_word & 0xFFFF) as u16,
            body: &data[8..message_len],
        })
    }

    /// Parse the next [`OwnedFd`] from the `Ctrl` buffer
    ///
    /// Returns `None` if there are none left in the buffer
    pub fn parse_fd(&mut self) -> Option<OwnedFd> {
        let ctrl_end = self.ctrl_end.unwrap_or_else(|| {
            // if there is no calculated end yet, just assume its the max length
            self.ctrl_buf.as_ref().len()
        });

        // loop here so we can retry when parsing non SCM_RIGHTS data
        loop {
            // get the section of the ctrl buffer that has remaining ctrl data in it
            let ctrl = &self.ctrl_buf.as_ref()[self.ctrl_start..ctrl_end];

            // return none if the buffer is not big enough to hold a cmsghdr
            if ctrl.len() < mem::size_of::<libc::cmsghdr>() {
                // this also means we are at the end and it can be set
                self.ctrl_end = Some(self.ctrl_start);
                return None;
            }

            // create a pointer from the ctrl buffer and read it as a cmsghdr
            let cmsg_ptr = ctrl.as_ptr() as *const libc::cmsghdr;
            let cmsghdr = unsafe { std::ptr::read_unaligned(cmsg_ptr) };

            // return none if the cmsg length is invalid
            if cmsghdr.cmsg_len < mem::size_of::<libc::cmsghdr>() {
                // this also means we are at the end and it can be set
                self.ctrl_end = Some(self.ctrl_start);
                return None;
            }

            // pad the cmsg length to the correct alignment
            let align_len = cmsg_align(cmsghdr.cmsg_len);

            // increment the ctrl start index for the next iteration
            self.ctrl_start += align_len;

            // ensure the cmsg_level represents a SCM_RIGHTS file descriptor
            if cmsghdr.cmsg_level != libc::SCM_RIGHTS {
                log::warn!("parsed non SCM_RIGHTS ctrl message from wayland buffer");
                continue;
            }

            // load the fd pointer from the cmsg data
            let fd_ptr = unsafe { cmsg_ptr.offset(1) as *const RawFd };
            let raw_fd = unsafe { core::ptr::read_unaligned(fd_ptr) };

            // then build and return the owned fd
            return Some(unsafe { OwnedFd::from_raw_fd(raw_fd) });
        }
    }

    /// Reads as many bytes from `stream` as possible
    ///
    /// Returns `true` if any data was received from the socket
    pub fn read_from_stream(&mut self, stream: &mut UnixStream) -> io::Result<bool> {
        // shift both buffers to make space for incoming data
        self.shift_data_buffer();
        self.shift_ctrl_buffer();

        // calculate the end of the ctrl buffer
        // this is required so that
        let ctrl_end = self.calculate_ctrl_end();

        // get the empty data and ctrl buffer sections
        let data = &mut self.data_buf.as_mut()[self.data_end..];
        let ctrl = &mut self.ctrl_buf.as_mut()[ctrl_end..];

        // ensure the ctrl buffer is zeroed out
        ctrl.fill(0);

        // build scatter/gather array with single data buffer
        let msg_iov = &mut [libc::iovec {
            iov_base: data.as_mut_ptr() as *mut _,
            iov_len: data.len(),
        }];

        // build msghdr for the recv call
        let mut msghdr = libc::msghdr {
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: msg_iov.as_mut_ptr(),
            msg_iovlen: 1,
            msg_control: ctrl.as_mut_ptr() as *mut _,
            msg_controllen: ctrl.len(),
            msg_flags: 0,
        };

        // call recvmsg to get data from the client
        let recv_len = unsafe {
            libc::recvmsg(
                stream.as_raw_fd(),
                (&mut msghdr) as *mut _,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        };

        // try to convert the received length into a valid data length
        let Ok(data_len) = usize::try_from(recv_len) else {
            return match io::Error::last_os_error() {
                // if we got a blocking error, just return false instead
                e if e.kind() == io::ErrorKind::WouldBlock => Ok(false),
                e => Err(e),
            };
        };

        // ensure no control data was truncated
        if msghdr.msg_flags & libc::MSG_CTRUNC > 0 {
            return Err(io::Error::other(
                "ctrl buffer overflow, file descriptors were truncated",
            ));
        }

        // return false if no data was read
        if data_len == 0 {
            return Ok(false);
        }

        // increment the data end and return true
        self.data_end += data_len;
        Ok(true)
    }

    fn shift_ctrl_buffer(&mut self) {
        // if the start position is at zero, then it is already shifted
        if self.ctrl_start == 0 {
            return;
        }

        // to shift only valid data we have to calculate the end of the ctrl_buffer
        let ctrl_end = self.calculate_ctrl_end();

        // if the start and end are equal, these can just be reset to zero
        if self.ctrl_start == ctrl_end {
            self.ctrl_start = 0;
            self.ctrl_end = Some(0);
        }
        // if they are not, copy the remaining data to the start of the buffer
        else {
            let ctrl = self.ctrl_buf.as_mut();
            ctrl.copy_within(self.ctrl_start..ctrl_end, 0);
            self.ctrl_end = Some(ctrl_end - self.ctrl_start);
            self.ctrl_start = 0;
        }
    }

    fn shift_data_buffer(&mut self) {
        // if the start position is at zero, then it is already shifted
        if self.data_start == 0 {
            return;
        }

        // if the start and end are equal, these can just be reset to zero
        if self.data_start == self.data_end {
            self.data_start = 0;
            self.data_end = 0;
        }
        // if they are not, copy the remaining data to the start of the buffer
        else {
            let data = self.data_buf.as_mut();
            data.copy_within(self.data_start..self.data_end, 0);
            self.data_end -= self.data_start;
            self.data_start = 0;
        }
    }

    fn calculate_ctrl_end(&mut self) -> usize {
        // if its already calculated, just return it
        if let Some(ctrl_end) = self.ctrl_end {
            return ctrl_end;
        }

        // initially assume that the end index is the same as the start
        let mut ctrl_end = self.ctrl_start;

        // loop and update the end location if cmsgs are found
        loop {
            // get the section of the ctrl buffer that has remaining data in it
            let remaining = &self.ctrl_buf.as_ref()[ctrl_end..];

            // break if the buffer is not big enough to hold a cmsghdr
            if remaining.len() < mem::size_of::<libc::cmsghdr>() {
                break;
            }

            // create a pointer from the ctrl buffer and read it as a cmsghdr
            let cmsg_ptr = remaining.as_ptr() as *const libc::cmsghdr;
            let cmsghdr = unsafe { std::ptr::read_unaligned(cmsg_ptr) };

            // break if the cmsg length is invalid
            if cmsghdr.cmsg_len < mem::size_of::<libc::cmsghdr>() {
                break;
            }

            // pad the cmsg length to the correct alignment
            let align_len = cmsg_align(cmsghdr.cmsg_len);

            // add the cmsg length to the end index and try again
            ctrl_end += align_len;
        }

        // store and return the calculated ctrl end
        self.ctrl_end = Some(ctrl_end);
        ctrl_end
    }
}

const fn cmsg_align(len: usize) -> usize {
    const USIZE_ALIGN: usize = mem::size_of::<usize>() - 1;
    len + USIZE_ALIGN & !USIZE_ALIGN
}

#[cfg(test)]
mod tests {
    use std::os::fd::IntoRawFd;

    use super::*;

    fn encode_fd(bytes: &mut Vec<u8>, fd: RawFd) {
        let mut cmsg_len = mem::size_of::<libc::cmsghdr>() + mem::size_of::<RawFd>();

        // build cmsghdr
        bytes.extend_from_slice(&cmsg_len.to_ne_bytes()); // cmsg_len
        bytes.extend_from_slice(&libc::SCM_RIGHTS.to_ne_bytes()); // cmsg_level
        bytes.extend_from_slice(&[0, 0, 0, 0]); // cmsg_type

        // insert the file descriptor
        bytes.extend_from_slice(&fd.to_ne_bytes());

        // pad to length
        let padded_len = cmsg_align(cmsg_len);
        while cmsg_len < padded_len {
            bytes.push(0);
            cmsg_len += 1;
        }
    }

    fn encode_message(bytes: &mut Vec<u8>, message: &Message) {
        // build the second word
        let mut message_len = (8 + message.body.len()) as u16;
        let second_word = (message.opcode as u32) | ((message_len as u32) << 16);

        // insert the message data
        bytes.extend_from_slice(&message.object_id.to_ne_bytes());
        bytes.extend_from_slice(&second_word.to_ne_bytes());
        bytes.extend_from_slice(message.body);

        // pad to length
        let padded_len = (message_len + 3) & !3;
        while message_len < padded_len {
            bytes.push(0);
            message_len += 1;
        }
    }

    #[test]
    fn parse_single_message() {
        const MESSAGE: Message = Message {
            object_id: 42,
            opcode: 69,
            body: &[1, 2, 3, 4, 5],
        };

        let mut bytes = Vec::new();
        encode_message(&mut bytes, &MESSAGE);
        let data_end = bytes.len();

        let mut buffer = ReadBuffer {
            data_buf: bytes,
            ctrl_buf: [],
            data_start: 0,
            ctrl_start: 0,
            data_end,
            ctrl_end: Some(0),
        };

        let message = buffer.parse_message().unwrap();
        assert_eq!(message.object_id, MESSAGE.object_id);
        assert_eq!(message.opcode, MESSAGE.opcode);
        assert_eq!(message.body, MESSAGE.body);

        assert!(buffer.parse_message().is_none());
    }

    #[test]
    fn parse_single_fd() {
        const RAW: RawFd = 42;

        let mut bytes = Vec::new();
        encode_fd(&mut bytes, RAW);
        let ctrl_end = bytes.len();

        let mut buffer = ReadBuffer {
            data_buf: [],
            ctrl_buf: bytes,
            data_start: 0,
            ctrl_start: 0,
            data_end: 0,
            ctrl_end: Some(ctrl_end),
        };

        let fd = buffer.parse_fd().unwrap().into_raw_fd();
        assert_eq!(fd, RAW);

        assert!(buffer.parse_fd().is_none());
    }

    #[test]
    fn parse_multi_message() {
        const COUNT: usize = 3;
        const MESSAGE: Message = Message {
            object_id: 42,
            opcode: 69,
            body: &[1, 2, 3, 4, 5],
        };

        let mut bytes = Vec::new();
        for _ in 0..COUNT {
            encode_message(&mut bytes, &MESSAGE);
        }
        let data_end = bytes.len();

        let mut buffer = ReadBuffer {
            data_buf: bytes,
            ctrl_buf: [],
            data_start: 0,
            ctrl_start: 0,
            data_end,
            ctrl_end: Some(0),
        };

        for _ in 0..COUNT {
            let message = buffer.parse_message().unwrap();
            assert_eq!(message.object_id, MESSAGE.object_id);
            assert_eq!(message.opcode, MESSAGE.opcode);
            assert_eq!(message.body, MESSAGE.body);
        }

        assert!(buffer.parse_message().is_none());
    }

    #[test]
    fn parse_multi_fd() {
        const COUNT: usize = 3;
        const RAW: RawFd = 42;

        let mut bytes = Vec::new();
        for _ in 0..COUNT {
            encode_fd(&mut bytes, RAW);
        }
        let ctrl_end = bytes.len();

        let mut buffer = ReadBuffer {
            data_buf: [],
            ctrl_buf: bytes,
            data_start: 0,
            ctrl_start: 0,
            data_end: 0,
            ctrl_end: Some(ctrl_end),
        };

        for _ in 0..COUNT {
            let fd = buffer.parse_fd().unwrap().into_raw_fd();
            assert_eq!(fd, RAW)
        }

        assert!(buffer.parse_fd().is_none());
    }

    #[test]
    fn parse_partial() {
        const MESSAGE: Message = Message {
            object_id: 42,
            opcode: 69,
            body: &[1, 2, 3, 4, 5],
        };

        let mut bytes = Vec::new();
        encode_message(&mut bytes, &MESSAGE);

        let mut buffer = ReadBuffer {
            data_buf: bytes,
            ctrl_buf: [],
            data_start: 0,
            ctrl_start: 0,
            data_end: 7,
            ctrl_end: Some(0),
        };

        assert!(buffer.parse_message().is_none());

        buffer.data_end = buffer.data_buf.len();
        let message = buffer.parse_message().unwrap();
        assert_eq!(message.object_id, MESSAGE.object_id);
        assert_eq!(message.opcode, MESSAGE.opcode);
        assert_eq!(message.body, MESSAGE.body);

        assert!(buffer.parse_message().is_none());
    }
}
