#[doc(hidden)]
pub mod __impl {
    pub use wayne_protocol_macros::*;
}

#[macro_export]
macro_rules! generate {
    ($path:literal) => {
        $crate::macros::__impl::protocol!($crate, $path);
    };
}
