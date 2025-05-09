use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct RawString(Box<[u8]>);

impl RawString {
    pub fn from_bytes(bytes: Box<[u8]>) -> Self {
        Self(bytes)
    }
}

impl Display for RawString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}
