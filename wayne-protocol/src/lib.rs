pub mod protocols;
pub mod types;

#[doc(hidden)]
pub mod __macro_impl {
    pub use wayne_protocol_macros::*;
}

#[macro_export]
macro_rules! generate {
    ($path:literal) => {
        $crate::__macro_impl::protocol!($crate, $path);
    };
}
