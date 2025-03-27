#[derive(Debug, Clone)]
pub struct Message {
    pub object_id: u32,
    pub opcode: u16,
    pub body: Box<[u8]>,
}
