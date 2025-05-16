pub mod core {
    pub use wayne_core::*;
}

pub mod protocol {
    pub use wayne_protocol::*;
}

pub mod stream {
    pub use wayne_stream::*;
}

pub use protocol::protocol;
pub use stream::WaylandStream;
