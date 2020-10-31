use alloc::{sync::Arc, vec::Vec};
use core::fmt::{Debug, Formatter, Result};

use spin::Mutex;

use super::{PmArea, VmArea};
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{align_down, align_up},
    Frame, MMUFlags, PhysAddr, VirtAddr, PAGE_SIZE,
};

/// A discontiguous PMA which perform delay allocation (e.g. in page fault handler).
pub struct PmAreaDelay {
    frames: Vec<Option<Frame>>,
}

impl PmArea for PmAreaDelay {
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
    fn read(&mut self, offset: usize, buf: &mut [u8]) -> AcoreResult<usize> {
        let mut total_len = 0;
        self.for_each_frame(offset, buf.len(), |frame: &mut [u8]| {
            let len = frame.len();
            buf[total_len..total_len + len].copy_from_slice(frame);
            total_len += len;
        })?;
        Ok(total_len)
    }
    fn write(&mut self, offset: usize, buf: &[u8]) -> AcoreResult<usize> {
        let mut total_len = 0;
        self.for_each_frame(offset, buf.len(), |frame: &mut [u8]| {
            let len = frame.len();
            frame.copy_from_slice(&buf[total_len..total_len + len]);
            total_len += len;
        })?;
        Ok(total_len)
    }
}

impl PmAreaDelay {
    pub fn new(page_count: usize) -> AcoreResult<Self> {
        if page_count == 0 {
            warn!(
                "page_count cannot be 0 in PmAreaDelay::new(): {:#x?}",
                page_count
            );
            return Err(AcoreError::InvalidArgs);
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
        mut op: impl FnMut(&mut [u8]),
    ) -> AcoreResult {
        if offset >= self.size() {
            warn!(
                "out of range in PmAreaDelay::for_each_frame(): offset={:#x?}, {:#x?}",
                offset, self
            );
            return Err(AcoreError::OutOfRange);
        }
        let len = len.min(self.size() - offset);
        let start_offset = offset - align_down(offset);
        let end_offset = align_up(offset + len) - (offset + len);
        let start_idx = offset / PAGE_SIZE;
        let end_idx = align_up(offset + len) / PAGE_SIZE;
        for i in start_idx..end_idx {
            let (mut range_start, mut range_end) = (0, PAGE_SIZE);
            if i == start_idx {
                range_start += start_offset;
            }
            if i == end_idx - 1 {
                range_end -= end_offset;
            }

            if self.frames[i].is_none() {
                let mut frame = Frame::new()?;
                frame.zero();
                self.frames[i] = Some(frame);
            }
            let frame = self.frames[i].as_mut().unwrap();
            op(&mut frame.as_slice_mut()[range_start..range_end]);
        }
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
