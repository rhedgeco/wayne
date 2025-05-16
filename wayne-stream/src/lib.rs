use std::{
    io, mem,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::net::UnixStream,
    },
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message<'a> {
    pub object_id: u32,
    pub opcode: u16,
    pub body: &'a [u8],
}

pub struct WaylandStream<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    stream: UnixStream,
    data_buf: Data,
    ctrl_buf: Ctrl,
    data_start: usize,
    ctrl_start: usize,
    data_end: usize,
}

impl<Data, Ctrl> WaylandStream<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    pub fn unix(stream: UnixStream) -> WaylandStreamBuilder<(), ()> {
        WaylandStreamBuilder {
            stream,
            data_buf: (),
            ctrl_buf: (),
        }
    }

    pub fn connect(path: impl AsRef<Path>) -> io::Result<WaylandStreamBuilder<(), ()>> {
        let stream = UnixStream::connect(path)?;
        Ok(Self::unix(stream))
    }
}

pub struct WaylandStreamBuilder<Data, Ctrl> {
    stream: UnixStream,
    data_buf: Data,
    ctrl_buf: Ctrl,
}

impl<Data, Ctrl> WaylandStreamBuilder<Data, Ctrl> {
    pub fn with_data_buffer<NewData>(self, buf: NewData) -> WaylandStreamBuilder<NewData, Ctrl>
    where
        NewData: AsRef<[u8]> + AsMut<[u8]>,
    {
        WaylandStreamBuilder {
            stream: self.stream,
            data_buf: buf,
            ctrl_buf: self.ctrl_buf,
        }
    }

    pub fn with_ctrl_buffer<NewCtrl>(self, buf: NewCtrl) -> WaylandStreamBuilder<Data, NewCtrl>
    where
        NewCtrl: AsRef<[u8]> + AsMut<[u8]>,
    {
        WaylandStreamBuilder {
            stream: self.stream,
            data_buf: self.data_buf,
            ctrl_buf: buf,
        }
    }

    pub fn build(self) -> WaylandStream<Data, Ctrl>
    where
        Data: AsRef<[u8]> + AsMut<[u8]>,
        Ctrl: AsRef<[u8]> + AsMut<[u8]>,
    {
        let data_len = self.data_buf.as_ref().len();
        let ctrl_len = self.ctrl_buf.as_ref().len();
        WaylandStream {
            stream: self.stream,
            data_buf: self.data_buf,
            ctrl_buf: self.ctrl_buf,
            data_start: data_len,
            ctrl_start: ctrl_len,
            data_end: data_len,
        }
    }
}

impl<Data, Ctrl> Drop for WaylandStream<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    fn drop(&mut self) {
        // the ctrl buffer must be cleared before dropping
        // this ensures there are no file descriptors left dangling
        self.clear_ctrl_buffer();
    }
}

impl<Data, Ctrl> WaylandStream<Data, Ctrl>
where
    Data: AsRef<[u8]> + AsMut<[u8]>,
    Ctrl: AsRef<[u8]> + AsMut<[u8]>,
{
    /// Clears all pending buffer data
    pub fn clear_buffers(&mut self) {
        self.clear_data_buffer();
        self.clear_ctrl_buffer();
    }

    /// Clears all pending `Data` buffer data
    pub fn clear_data_buffer(&mut self) {
        // parse all remaining messages
        while self.parse_message().is_some() {}

        if self.data_start < self.data_end {
            // if there is remaining data, copy it to the start of the buffer
            let buffer = self.data_buf.as_mut();
            buffer.copy_within(self.data_start..self.data_end, 0);
            self.data_end -= self.data_start;
            self.data_start = 0;
        } else {
            // otherwise reset the start and end indices to zero
            self.data_start = 0;
            self.data_end = 0;
        }
    }

    /// Clears all pending `Ctrl` buffer data
    pub fn clear_ctrl_buffer(&mut self) {
        // parse all remaining file descriptors
        while self.parse_fd().is_some() {}

        // zero out all the bytes in the buffer
        for byte in self.ctrl_buf.as_mut() {
            *byte = 0;
        }

        // reset the ctrl start index to zero
        self.ctrl_start = 0;
    }

    /// Parse the next [`Message`] in the `Data` buffer
    ///
    /// Returns `None` if there are no more messages
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
        let message_len = ((second_word >> 16) as u16).max(8);

        // pad message length to align to multiple of 4 (32 bits)
        let message_len = ((message_len + 3) & !3) as usize;

        // ensure there is enough data for the rest of the message
        if data.len() < message_len {
            return None;
        }

        // increment the data start index for the next iteration
        self.data_start = self.data_start.saturating_add(message_len);

        // build and return the parsed message
        Some(Message {
            object_id: u32::from_ne_bytes([data[0], data[1], data[2], data[3]]),
            opcode: (second_word & 0xFFFF) as u16,
            body: &data[8..message_len],
        })
    }

    /// Parse the next [`OwnedFd`] in the `Ctrl` buffer
    ///
    /// Returns `None` if there are no more messages
    pub fn parse_fd(&mut self) -> Option<OwnedFd> {
        // loop here so we can skip parsing invalid cmsg data
        loop {
            // get the section of the ctrl buffer that has remaining ctrl data in it
            let ctrl = &self.ctrl_buf.as_ref()[self.ctrl_start..];

            // return none if the buffer is not big enough to hold a cmsghdr
            if ctrl.len() < mem::size_of::<libc::cmsghdr>() {
                return None;
            }

            // create a pointer from the ctrl buffer and read it as a cmsghdr
            let cmsg_ptr = ctrl.as_ptr() as *const libc::cmsghdr;
            let cmsghdr = unsafe { std::ptr::read_unaligned(cmsg_ptr) };

            // ensure the cmsg_len is valid
            if cmsghdr.cmsg_len < mem::size_of::<libc::cmsghdr>() {
                // if it is not, then there is no more cmsg data
                // set the ctrl_start to the end of the buffer and return none
                self.ctrl_start = self.ctrl_buf.as_ref().len();
                return None;
            }

            // get the cmsg length and pad it to the correct alignment
            const USIZE_ALIGN: usize = mem::size_of::<usize>() - 1;
            let align_len = cmsghdr.cmsg_len + USIZE_ALIGN & !USIZE_ALIGN;

            // increment the ctrl start index for the next iteration
            self.ctrl_start = self.ctrl_start.saturating_add(align_len);

            // ensure the cmsg_level represents a file descriptor
            if cmsghdr.cmsg_level != libc::SCM_RIGHTS {
                log::warn!("received non SCM_RIGHTS ctrl message in wayland stream");
                continue;
            }

            // load the fd pointer from the cmsg data
            let fd_ptr = unsafe { cmsg_ptr.offset(1) as *const RawFd };
            let raw_fd = unsafe { core::ptr::read_unaligned(fd_ptr) };

            // then build and return the owned fd
            return Some(unsafe { OwnedFd::from_raw_fd(raw_fd) });
        }
    }

    /// Clear buffers and reads as much new data from the wayland socket as possible
    ///
    /// Returns `false` if no message data was receieved
    pub fn read_socket(&mut self) -> io::Result<bool> {
        // clear pending buffer data before reading
        self.clear_buffers();

        // get the relevant buffer sections to be filled
        let data_buf = &mut self.data_buf.as_mut()[self.data_end..];
        let ctrl_buf = self.ctrl_buf.as_mut();

        // build scatter/gather array with single buffer
        let msg_iov = &mut [libc::iovec {
            iov_base: data_buf.as_mut_ptr() as *mut _,
            iov_len: data_buf.len(),
        }];

        // build msghdr for the recv call
        let mut msghdr = libc::msghdr {
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: msg_iov.as_mut_ptr(),
            msg_iovlen: 1,
            msg_control: ctrl_buf.as_mut_ptr() as *mut _,
            msg_controllen: ctrl_buf.len(),
            msg_flags: 0,
        };

        // call recvmsg to get data from the client
        // set data_end to the filled buffer length
        let data_len = match unsafe {
            libc::recvmsg(
                self.stream.as_raw_fd(),
                (&mut msghdr) as *mut _,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        } {
            ..0 => match io::Error::last_os_error() {
                e if e.kind() == io::ErrorKind::WouldBlock => 0,
                e => return Err(e),
            },
            data_len => data_len as usize,
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

        // increment the data length and return true
        self.data_end += data_len;
        Ok(true)
    }
}
