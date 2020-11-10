use core::fmt::{Debug, Formatter, Result};
use core::ops::{Deref, DerefMut};

pub const CACHE_LINE_SIZE: usize = 64;

#[repr(align(64))]
pub struct AlignCacheLine<T>(T);

impl<T> AlignCacheLine<T> {
    pub const fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: Copy> AlignCacheLine<T> {
    pub fn get(&self) -> T {
        self.0
    }
}

impl<T> Deref for AlignCacheLine<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for AlignCacheLine<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> AsRef<T> for AlignCacheLine<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for AlignCacheLine<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> From<T> for AlignCacheLine<T> {
    fn from(t: T) -> Self {
        Self::new(t)
    }
}

impl<T: Debug> Debug for AlignCacheLine<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0.fmt(f)
    }
}

pub fn is_cache_line_aligned(offset: usize) -> bool {
    offset & (CACHE_LINE_SIZE - 1) == 0
}

pub fn alignup_cache_line(offset: usize) -> usize {
    (offset + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)
}
