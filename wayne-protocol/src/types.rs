macro_rules! newtype {
    ($outer:ident: $inner:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $outer($inner);

        impl $outer {
            pub fn new(value: $inner) -> Self {
                Self(value)
            }

            pub fn value(self) -> $inner {
                self.0
            }
        }
    };
}

newtype!(ObjectId: u32);
newtype!(NewId: u32);
newtype!(Opcode: u16);
