use alloc::{sync::Arc, vec::Vec};
use core::fmt::{Debug, Formatter, Result};

use spin::Mutex;

use super::{PmArea, VmArea};
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{self, align_down},
    Frame, MMUFlags, PhysAddr, VirtAddr, PAGE_SIZE, USER_VIRT_ADDR_LIMIT,
};

/// A discontiguous PMA which perform lazy allocation (e.g. in page fault handler).
pub struct PmAreaLazy {
    frames: Vec<Option<Frame>>,
}

impl PmArea for PmAreaLazy {
    fn size(&self) -> usize {
        self.frames.len() * PAGE_SIZE
    }
    fn get_frame(&mut self, idx: usize, need_alloc: bool) -> AcoreResult<Option<PhysAddr>> {
        if need_alloc && self.frames[idx].is_none() {
            let mut frame = Frame::new()?;
            frame.zero();
            self.frames[idx] = Some(frame);
        }
        Ok(self.frames[idx].as_ref().map(|f| f.start_paddr()))
    }
    fn release_frame(&mut self, idx: usize) -> AcoreResult {
        self.frames[idx].take().ok_or(AcoreError::NotFound)?;
        Ok(())
    }
    fn read(&mut self, offset: usize, dst: &mut [u8]) -> AcoreResult<usize> {
        self.for_each_frame(offset, dst.len(), |processed: usize, frame: &mut [u8]| {
            dst[processed..processed + frame.len()].copy_from_slice(frame);
        })
    }
    fn write(&mut self, offset: usize, src: &[u8]) -> AcoreResult<usize> {
        self.for_each_frame(offset, src.len(), |processed: usize, frame: &mut [u8]| {
            frame.copy_from_slice(&src[processed..processed + frame.len()]);
        })
    }
}

impl PmAreaLazy {
    pub fn new(page_count: usize) -> AcoreResult<Self> {
        if page_count == 0 {
            warn!(
                "page_count cannot be 0 in PmAreaLazy::new(): {:#x?}",
                page_count
            );
            return Err(AcoreError::InvalidArgs);
        }
        if page_count > addr::page_count(USER_VIRT_ADDR_LIMIT) {
            warn!(
                "page_count is too large in PmAreaLazy::new(): {:#x?}",
                page_count
            );
            return Err(AcoreError::NoMemory);
        }
        let mut frames = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            frames.push(None);
        }
        Ok(Self { frames })
    }

    fn for_each_frame(
        &mut self,
        offset: usize,
        len: usize,
        mut op: impl FnMut(usize, &mut [u8]),
    ) -> AcoreResult<usize> {
        if offset >= self.size() || offset + len > self.size() {
            warn!(
                "out of range in PmAreaLazy::for_each_frame(): offset={:#x?}, len={:#x?}, {:#x?}",
                offset, len, self
            );
            return Err(AcoreError::OutOfRange);
        }
        let mut start = offset;
        let mut len = len;
        let mut processed = 0;
        while len > 0 {
            let start_align = align_down(start);
            let pgoff = start - start_align;
            let n = (PAGE_SIZE - pgoff).min(len);

            let idx = start_align / PAGE_SIZE;
            if self.frames[idx].is_none() {
                let mut frame = Frame::new()?;
                frame.zero();
                self.frames[idx] = Some(frame);
            }
            let frame = self.frames[idx].as_mut().unwrap();
            op(processed, &mut frame.as_slice_mut()[pgoff..pgoff + n]);
            start += n;
            processed += n;
            len -= n;
        }
        Ok(processed)
    }
}

impl Debug for PmAreaLazy {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("PmAreaLazy")
            .field("size", &self.size())
            .finish()
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
            Arc::new(Mutex::new(PmAreaLazy::new(size)?)),
            name,
        )
    }
}
