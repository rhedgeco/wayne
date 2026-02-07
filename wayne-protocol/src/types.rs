#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewId(pub u32);

impl PartialEq<ObjectId> for NewId {
    fn eq(&self, other: &ObjectId) -> bool {
        self.0 == other.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub u32);

impl PartialEq<NewId> for ObjectId {
    fn eq(&self, other: &NewId) -> bool {
        self.0 == other.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed(pub i32);

impl Fixed {
    pub fn to_f32(&self) -> f32 {
        self.0 as f32 / 256f32
    }

    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / 256f64
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawString<'a>(pub &'a [u8]);

impl<'a> RawString<'a> {
    pub fn is_null(&self) -> bool {
        self.0.len() == 0
    }

    pub fn has_interior_nulls(&self) -> bool {
        let interior_len = self.0.len().saturating_sub(1);
        (&self.0[0..interior_len]).iter().any(|v| *v == 0)
    }

    pub fn null_terminated(&self) -> bool {
        match self.0.last() {
            Some(value) => *value == 0,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_comparison() {
        const VALUE: u32 = 10;
        let object_id = ObjectId(VALUE);
        let new_id = NewId(VALUE);
        assert_eq!(object_id, new_id);
        assert_eq!(new_id, object_id);
    }

    #[test]
    fn fixed_to_float() {
        // fixed value should be Signed 24.8 decimal numbers
        // https://wayland.freedesktop.org/docs/html/ch04.html#sect-Protocol-Wire-Format

        // whole numbers
        assert_eq!(Fixed(256).to_f32(), 1.0);
        assert_eq!(Fixed(512).to_f64(), 2.0);
        assert_eq!(Fixed(0).to_f32(), 0.0);

        // fractional numbers
        assert_eq!(Fixed(1).to_f32(), 1.0 / 256.0);
        assert_eq!(Fixed(-128).to_f32(), -0.5);
    }

    #[test]
    fn raw_string_null_check() {
        let null_string = RawString(&[]);
        assert!(null_string.is_null());

        let non_null_string = RawString(&[1]);
        assert!(!non_null_string.is_null());
    }

    #[test]
    fn raw_string_interior_null_check() {
        let interior_nulls = RawString(&[1, 0, 3, 0]);
        assert!(interior_nulls.has_interior_nulls());

        let no_interior_nulls = RawString(&[1, 2, 3, 0]);
        assert!(!no_interior_nulls.has_interior_nulls());
    }

    #[test]
    fn raw_string_null_terminated() {
        let null_terminated = RawString(&[1, 2, 0]);
        assert!(null_terminated.null_terminated());

        let not_null_terminated = RawString(&[1, 2, 3]);
        assert!(!not_null_terminated.null_terminated());
    }
}
