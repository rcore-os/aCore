use alloc::sync::Arc;

use spin::Mutex;

use super::{PmArea, VmArea};
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{align_down, align_up},
    MMUFlags, PhysAddr,
};

/// A PMA representing a fixed physical memory region.
#[derive(Debug)]
pub struct PmAreaFixed {
    start: PhysAddr,
    end: PhysAddr,
}

impl PmArea for PmAreaFixed {
    fn size(&self) -> usize {
        self.end - self.start
    }
    fn get_frame(&mut self, offset: usize, _need_alloc: bool) -> AcoreResult<Option<PhysAddr>> {
        if offset >= self.size() {
            warn!(
                "out of range in PmAreaFixed::get_frame(): offset={:#x?}, {:#x?}",
                offset, self
            );
            return Err(AcoreError::OutOfRange);
        }
        Ok(Some(align_down(self.start + offset)))
    }
    fn release_frame(&mut self, _offset: usize) -> AcoreResult {
        Ok(())
    }
}

impl PmAreaFixed {
    pub fn new(start: PhysAddr, end: PhysAddr) -> AcoreResult<Self> {
        if start >= end {
            warn!(
                "invalid memory region in PmAreaFixed::new(): [{:#x?}, {:#x?})",
                start, end
            );
            return Err(AcoreError::InvalidArgs);
        }
        Ok(Self {
            start: align_down(start),
            end: align_up(end),
        })
    }
}

impl VmArea {
    pub fn from_fixed_pma(
        start_paddr: PhysAddr,
        end_paddr: PhysAddr,
        offset: usize,
        flags: MMUFlags,
        name: &'static str,
    ) -> AcoreResult<Self> {
        Self::new(
            start_paddr + offset,
            end_paddr + offset,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_paddr, end_paddr)?)),
            name,
        )
    }
}
