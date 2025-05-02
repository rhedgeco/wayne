#[doc(hidden)]
pub mod __impl {
    pub use wayne_protocol_macros::protocol;
}

#[macro_export]
macro_rules! protocol {
    ($path:literal) => {
        $crate::macros::__impl::protocol!($crate, $path);
    };
}
