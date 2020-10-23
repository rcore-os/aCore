use core::ops::Range;

use bitmap_allocator::{BitAlloc, BitAlloc64K};

use crate::error::{AcoreError, AcoreResult};

pub struct IdAllocator {
    inner: BitAlloc64K,
}

impl IdAllocator {
    pub fn new(range: Range<usize>) -> Self {
        let mut inner = BitAlloc64K::DEFAULT;
        inner.insert(range);
        Self { inner }
    }

    pub fn alloc(&mut self) -> AcoreResult<usize> {
        self.inner.alloc().ok_or(AcoreError::NoResources)
    }

    pub fn dealloc(&mut self, id: usize) {
        self.inner.dealloc(id)
    }
}
