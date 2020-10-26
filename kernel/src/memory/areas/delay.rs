use alloc::{sync::Arc, vec::Vec};
use core::fmt::{Debug, Formatter, Result};

use spin::Mutex;

use super::{PmArea, VmArea};
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{is_aligned, page_count},
    Frame, MMUFlags, PhysAddr, VirtAddr, PAGE_SIZE,
};

/// A discontiguous PMA which perform allocation in page fault handler.
pub struct PmAreaDelay {
    frames: Vec<Option<Frame>>,
}

impl PmArea for PmAreaDelay {
    fn size(&self) -> usize {
        self.frames.len() * PAGE_SIZE
    }
    fn get_frame(&mut self, offset: usize, need_alloc: bool) -> AcoreResult<Option<PhysAddr>> {
        let idx = offset / PAGE_SIZE;
        debug_assert!(idx < self.frames.len());
        if need_alloc {
            self.frames[idx] = Some(Frame::new()?);
        }
        Ok(self.frames[idx].as_ref().map(|f| f.start_paddr()))
    }
    fn release_frame(&mut self, offset: usize) -> AcoreResult {
        self.frames[offset / PAGE_SIZE]
            .take()
            .ok_or(AcoreError::NotFound)?;
        Ok(())
    }
}

impl Debug for PmAreaDelay {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("PmAreaDelay")
            .field("size", &self.size())
            .finish()
    }
}

impl PmAreaDelay {
    pub fn new(size: usize) -> AcoreResult<Self> {
        if size == 0 || !is_aligned(size) {
            warn!("invalid PMA size in PmAreaDelay::new(): {:#x?}", size);
            return Err(AcoreError::InvalidArgs);
        }
        let count = page_count(size);
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            frames.push(None);
        }
        Ok(Self { frames })
    }

    pub fn pre_alloc(&mut self, offset: usize, size: usize) -> AcoreResult {
        if !is_aligned(offset) {
            warn!(
                "offset not aligned in PmAreaDelay::pre_alloc(): {:#x?}",
                offset
            );
            return Err(AcoreError::InvalidArgs);
        }
        let count = page_count(size);
        let idx = offset / PAGE_SIZE;
        if idx + count > self.frames.len() {
            warn!(
                "out of range in PmAreaDelay::pre_alloc(): offset={:#x?}, size={:#x?}, {:#x?}",
                offset, size, self
            );
            return Err(AcoreError::OutOfRange);
        }
        for i in 0..count {
            self.frames[idx + i] = Some(Frame::new()?);
        }
        Ok(())
    }
}

impl VmArea {
    pub fn from_delay_pma(
        start_vaddr: VirtAddr,
        size: usize,
        flags: MMUFlags,
        name: &'static str,
    ) -> AcoreResult<Self> {
        Self::new(
            start_vaddr,
            start_vaddr + size,
            flags,
            Arc::new(Mutex::new(PmAreaDelay::new(size)?)),
            name,
        )
    }
}
