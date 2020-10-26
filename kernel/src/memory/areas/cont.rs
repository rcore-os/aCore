use alloc::sync::Arc;

use spin::Mutex;

use super::{PmArea, VmArea};
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{align_down, is_aligned, page_count},
    Frame, MMUFlags, PhysAddr, VirtAddr,
};

/// A contiguous PMA allocated at construction.
#[derive(Debug)]
pub struct PmAreaContiguous {
    frame: Frame,
}

impl PmArea for PmAreaContiguous {
    fn size(&self) -> usize {
        self.frame.size()
    }
    fn get_frame(&mut self, offset: usize, _need_alloc: bool) -> AcoreResult<Option<PhysAddr>> {
        debug_assert!(offset < self.size());
        Ok(Some(align_down(self.frame.start_paddr() + offset)))
    }
    fn release_frame(&mut self, _offset: usize) -> AcoreResult {
        Ok(())
    }
}

impl PmAreaContiguous {
    pub fn new(size: usize) -> AcoreResult<Self> {
        if size == 0 || !is_aligned(size) {
            warn!("invalid PMA size in PmAreaContiguous::new(): {:#x?}", size);
            return Err(AcoreError::InvalidArgs);
        }
        Ok(Self {
            frame: Frame::new_contiguous(page_count(size), 0)?,
        })
    }
}

impl VmArea {
    pub fn from_contiguous_pma(
        start_vaddr: VirtAddr,
        size: usize,
        flags: MMUFlags,
        name: &'static str,
    ) -> AcoreResult<Self> {
        Self::new(
            start_vaddr,
            start_vaddr + size,
            flags,
            Arc::new(Mutex::new(PmAreaContiguous::new(size)?)),
            name,
        )
    }
}
