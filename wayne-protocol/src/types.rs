macro_rules! newtype {
    ($inner:ident => $outer:ident) => {
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

newtype!(u32 => ObjectId);
newtype!(u32 => NewId);
newtype!(u16 => Opcode);
