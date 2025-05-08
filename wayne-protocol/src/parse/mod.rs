mod bytes;
mod stash;

pub use stash::Stash;

pub mod int;
pub const fn int() -> int::Parser {
    int::Parser::new()
}

pub mod uint;
pub const fn uint() -> uint::Parser {
    uint::Parser::new()
}

pub mod fixed;
pub const fn fixed() -> fixed::Parser {
    fixed::Parser::new()
}

pub mod string;
pub const fn string() -> string::Parser {
    string::Parser::new()
}

pub mod raw_id;
pub const fn raw_id() -> raw_id::Parser {
    raw_id::Parser::new()
}

pub mod array;
pub const fn array() -> array::Parser {
    array::Parser::new()
}

pub mod fd;
pub const fn fd() -> fd::Parser {
    fd::Parser::new()
}
