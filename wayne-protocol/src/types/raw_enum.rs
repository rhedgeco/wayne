use derivative::Derivative;

#[derive(Debug, Derivative)]
#[derivative(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct RawEnum<I, E> {
    #[derivative(Ord = "ignore")]
    #[derivative(PartialEq = "ignore")]
    #[derivative(PartialOrd = "ignore")]
    #[derivative(Hash = "ignore")]
    map: fn(&I) -> Option<E>,
    value: I,
}

impl<I, E> RawEnum<I, E> {
    pub fn build(&self) -> Option<E> {
        (self.map)(&self.value)
    }
}

impl<E: TryFrom<u32>> RawEnum<u32, E> {
    pub fn from_u32(value: u32) -> Self {
        Self {
            map: |value| E::try_from(*value).ok(),
            value,
        }
    }
}

impl<E: TryFrom<u32>> RawEnum<i32, E> {
    pub fn from_i32(value: i32) -> Self {
        Self {
            map: |value| {
                let value = u32::try_from(*value).ok()?;
                E::try_from(value).ok()
            },
            value,
        }
    }
}

impl<E: TryFrom<u32>> From<u32> for RawEnum<u32, E> {
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl<E: TryFrom<u32>> From<i32> for RawEnum<i32, E> {
    fn from(value: i32) -> Self {
        Self::from_i32(value)
    }
}
