//! Page table helpers.

use super::addr::{is_aligned, PhysAddr};
use super::vmm::VmArea;
use super::PAGE_SIZE;

pub use crate::arch::paging::{PageTable, PageTableFlags};

#[derive(Debug)]
pub enum PagingError {
    NoEntry,
    MapError,
    UnmapError,
}

pub type PagingResult<T = ()> = Result<T, PagingError>;

/// A wrapper of page table for map/unmap memory areas.
pub(super) struct Mapper {
    pub(super) pgtable: PageTable,
}

impl Mapper {
    pub fn new(pgtable: PageTable) -> Self {
        Self { pgtable }
    }

    pub fn map(&mut self, vma: &VmArea, target: PhysAddr) {
        trace!("create mapping: {:#x?} -> target {:#x?}", vma, target);
        debug_assert!(is_aligned(target));
        for vaddr in (vma.start..vma.end).step_by(PAGE_SIZE) {
            let paddr = vaddr - vma.start + target;
            self.pgtable
                .map(vaddr, paddr, vma.flags)
                .map_err(|e| {
                    panic!(
                        "failed to create mapping: {:#x?} -> {:#x?}, {:?}",
                        vaddr, paddr, e
                    )
                })
                .unwrap()
        }
    }

    pub fn unmap(&mut self, vma: &VmArea) {
        trace!("destory mapping: {:#x?}", vma);
        for vaddr in (vma.start..vma.end).step_by(PAGE_SIZE) {
            self.pgtable
                .unmap(vaddr)
                .map_err(|e| panic!("failed to unmap VA: {:#x?}, {:?}", vaddr, e))
                .unwrap()
        }
    }
}
