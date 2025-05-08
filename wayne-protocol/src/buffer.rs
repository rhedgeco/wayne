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

pub struct IterBuf<'a, I> {
    inner: &'a mut I,
}

impl<'a, I, T> Buffer<T> for IterBuf<'a, I>
where
    I: Iterator<Item = T>,
{
    fn take(&mut self) -> Option<T> {
        self.inner.next()
    }
}

impl<I: Iterator> IterBufExt for I {}
pub trait IterBufExt: Iterator {
    fn buffer(&mut self) -> IterBuf<Self>
    where
        Self: Sized,
    {
        IterBuf { inner: self }
    }
}
