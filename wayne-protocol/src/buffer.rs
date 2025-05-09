use std::collections::VecDeque;

pub trait Buffer<T> {
    fn take(&mut self) -> Option<T>;
}

impl<T, B: Buffer<T>> Buffer<T> for &mut B {
    fn take(&mut self) -> Option<T> {
        B::take(self)
    }
}

impl<T> Buffer<T> for VecDeque<T> {
    fn take(&mut self) -> Option<T> {
        self.pop_front()
    }
}

pub struct IterBuf<I>(I);

impl<T, I: Iterator<Item = T>> Buffer<T> for IterBuf<I> {
    fn take(&mut self) -> Option<T> {
        self.0.next()
    }
}

impl<T: Iterator> IterExt for T {}
pub trait IterExt: Iterator {
    fn buffer(self) -> IterBuf<Self>
    where
        Self: Sized,
    {
        IterBuf(self)
    }
}
