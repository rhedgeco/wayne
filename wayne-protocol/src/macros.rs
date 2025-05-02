#[doc(hidden)]
pub mod __impl {
    pub use wayne_protocol_macros::protocol;
}

/// Takes the path to a wayland protocol xml file, and generates the associated rust structrues.
///
/// The xml path is relative to your crates root directory.
#[macro_export]
macro_rules! protocol {
    ($path:literal) => {
        $crate::macros::__impl::protocol!($crate, $path);
    };
}
