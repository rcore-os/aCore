use alloc::sync::Arc;
use core::slice;

use spin::Mutex;

use super::{PmArea, VmArea};
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{align_down, align_up},
    MMUFlags, PhysAddr, PAGE_SIZE,
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
    fn get_frame(&mut self, idx: usize, _need_alloc: bool) -> AcoreResult<Option<PhysAddr>> {
        let paddr = self.start + idx * PAGE_SIZE;
        debug_assert!(paddr < self.end);
        Ok(Some(paddr))
    }
    fn release_frame(&mut self, _idx: usize) -> AcoreResult {
        Ok(())
    }
    fn read(&mut self, offset: usize, dst: &mut [u8]) -> AcoreResult<usize> {
        if offset >= self.size() {
            warn!(
                "out of range in PmAreaFixed::read(): offset={:#x?}, {:#x?}",
                offset, self
            );
            return Err(AcoreError::OutOfRange);
        }
        let len = dst.len().min(self.end - offset);
        let data = unsafe { slice::from_raw_parts((self.start + offset) as *const u8, len) };
        dst.copy_from_slice(data);
        Ok(len)
    }
    fn write(&mut self, offset: usize, src: &[u8]) -> AcoreResult<usize> {
        if offset >= self.size() {
            warn!(
                "out of range in PmAreaFixed::write(): offset={:#x?}, {:#x?}",
                offset, self
            );
            return Err(AcoreError::OutOfRange);
        }
        let len = src.len().min(self.end - offset);
        let data = unsafe { slice::from_raw_parts_mut((self.start + offset) as *mut u8, len) };
        data.copy_from_slice(src);
        Ok(len)
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
