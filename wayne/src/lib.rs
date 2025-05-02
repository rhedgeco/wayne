// re-export wayne_core under alias core
pub mod core {
    pub use wayne_core::*;
}

// re-export wayne_protocol under alias protocol
pub mod protocol {
    pub use wayne_protocol::*;
}

// re-export protocol macro at root level
pub use protocol::protocol;
