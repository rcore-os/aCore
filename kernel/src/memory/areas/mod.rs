mod fixed;
mod lazy;

pub use fixed::PmAreaFixed;
pub use lazy::PmAreaLazy;

use alloc::sync::Arc;

use spin::Mutex;

use super::addr::{align_down, align_up, PhysAddr, VirtAddr};
use super::paging::{MMUFlags, PageTable};
use super::PAGE_SIZE;
use crate::error::{AcoreError, AcoreResult};

/// A physical memory area with same MMU flags, can be discontiguous and lazy allocated,
/// or shared by multi-threads.
pub trait PmArea: core::fmt::Debug + Send + Sync {
    /// Size of total physical memory.
    fn size(&self) -> usize;
    /// Get the start address of a 4KB physical frame relative to the index `idx`, perform
    /// allocation if `need_alloc` is `true`.
    fn get_frame(&mut self, idx: usize, need_alloc: bool) -> AcoreResult<Option<PhysAddr>>;
    /// Release the given 4KB physical frame, perform deallocation if the frame has been allocated.
    fn release_frame(&mut self, idx: usize) -> AcoreResult;
    /// Read data from this PMA at `offset`.
    fn read(&mut self, offset: usize, dst: &mut [u8]) -> AcoreResult<usize>;
    /// Write data to this PMA at `offset`.
    fn write(&mut self, offset: usize, src: &[u8]) -> AcoreResult<usize>;
}

/// A contiguous virtual memory area with same MMU flags.
/// The `start` and `end` address are page aligned.
#[derive(Debug)]
pub struct VmArea {
    pub(super) start: VirtAddr,
    pub(super) end: VirtAddr,
    pub(super) flags: MMUFlags,
    pub(super) pma: Arc<Mutex<dyn PmArea>>,
    name: &'static str,
}

impl VmArea {
    pub fn new(
        start: VirtAddr,
        end: VirtAddr,
        flags: MMUFlags,
        pma: Arc<Mutex<dyn PmArea>>,
        name: &'static str,
    ) -> AcoreResult<Self> {
        if start >= end {
            warn!("invalid memory region: [{:#x?}, {:#x?})", start, end);
            return Err(AcoreError::InvalidArgs);
        }
        let start = align_down(start);
        let end = align_up(end);
        if end - start != pma.lock().size() {
            warn!(
                "VmArea size != PmArea size: [{:#x?}, {:#x?}), {:x?}",
                start,
                end,
                pma.lock()
            );
            return Err(AcoreError::InvalidArgs);
        }
        Ok(Self {
            start,
            end,
            flags,
            pma,
            name,
        })
    }

    /// Test whether a virtual address is contained in the memory area.
    pub fn contains(&self, vaddr: VirtAddr) -> bool {
        self.start <= vaddr && vaddr < self.end
    }

    /// Test whether this area is (page) overlap with region [`start`, `end`).
    pub fn is_overlap_with(&self, start: VirtAddr, end: VirtAddr) -> bool {
        let p0 = self.start;
        let p1 = self.end;
        let p2 = align_down(start);
        let p3 = align_up(end);
        !(p1 <= p2 || p0 >= p3)
    }

    /// Create mapping between this VMA to the associated PMA.
    pub fn map_area(&self, pt: &mut impl PageTable) -> AcoreResult {
        trace!("create mapping: {:#x?}", self);
        let mut pma = self.pma.lock();
        for vaddr in (self.start..self.end).step_by(PAGE_SIZE) {
            let page = pma.get_frame((vaddr - self.start) / PAGE_SIZE, false)?;
            let res = if let Some(paddr) = page {
                pt.map(vaddr, paddr, self.flags)
            } else {
                pt.map(vaddr, 0, MMUFlags::empty())
            };
            res.map_err(|e| {
                error!(
                    "failed to create mapping: {:#x?} -> {:#x?}, {:?}",
                    vaddr, page, e
                );
                e
            })?;
        }
        Ok(())
    }

    /// Destory mapping of this VMA.
    pub fn unmap_area(&self, pt: &mut impl PageTable) -> AcoreResult {
        trace!("destory mapping: {:#x?}", self);
        let mut pma = self.pma.lock();
        for vaddr in (self.start..self.end).step_by(PAGE_SIZE) {
            let res = pma.release_frame((vaddr - self.start) / PAGE_SIZE);
            if res != Err(AcoreError::NotFound) {
                if res.is_err() {
                    return res;
                }
                pt.unmap(vaddr).map_err(|e| {
                    error!("failed to unmap VA: {:#x?}, {:?}", vaddr, e);
                    e
                })?;
            }
        }
        Ok(())
    }

    /// Handle page fault.
    pub fn handle_page_fault(
        &self,
        offset: usize,
        access_flags: MMUFlags,
        pt: &mut impl PageTable,
    ) -> AcoreResult {
        debug_assert!(offset < self.end - self.start);
        trace!(
            "handle page fault @ offset {:#x?} with access {:?}: {:#x?}",
            offset,
            access_flags,
            self
        );
        let mut pma = self.pma.lock();
        if !self.flags.contains(access_flags) {
            return Err(AcoreError::AccessDenied);
        }
        let offset = align_down(offset);
        let vaddr = self.start + offset;
        let paddr = pma
            .get_frame(offset / PAGE_SIZE, true)?
            .ok_or(AcoreError::NoMemory)?;

        let entry = pt.get_entry(vaddr)?;
        if entry.is_present() {
            Err(AcoreError::AlreadyExists)
        } else {
            entry.set_addr(paddr);
            entry.set_flags(self.flags);
            pt.flush_tlb(Some(vaddr));
            Ok(())
        }
    }
}
