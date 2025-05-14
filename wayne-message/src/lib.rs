pub mod buffer;

pub use buffer::MessageBuffer;

/// A raw wayland message
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message<'a> {
    pub object_id: u32,
    pub opcode: u16,
    pub body: &'a [u8],
}
