//! Architecture independent page table traits and helpers.

use core::mem::ManuallyDrop;

use super::addr::{is_aligned, PhysAddr, VirtAddr};
use super::vmm::VmArea;
use super::PAGE_SIZE;
use crate::error::AcoreResult;

bitflags! {
    pub struct MMUFlags: usize {
        const DEVICE    = 1 << 0;
        const READ      = 1 << 1;
        const WRITE     = 1 << 2;
        const EXECUTE   = 1 << 3;
        const USER      = 1 << 4;
    }
}

pub trait PageTableEntry {
    fn addr(&self) -> PhysAddr;
    fn flags(&self) -> MMUFlags;

    fn set_addr(&mut self, paddr: PhysAddr);
    fn set_flags(&mut self, flags: MMUFlags);
    fn clear(&mut self);
}

pub trait PageTable: Sized {
    fn new() -> Self;

    /// Constructs a multi-level page table from a physical address of the root page table.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the user must ensure that the page table indicated by the
    /// memory region starting from `root_paddr` must has the correct format.
    unsafe fn from_root(root_paddr: PhysAddr) -> ManuallyDrop<Self>;

    fn current_root_paddr() -> PhysAddr;

    /// # Safety
    ///
    /// This function is unsafe because it switches the virtual address space.
    unsafe fn set_current_root_paddr(root_paddr: PhysAddr);

    fn flush_tlb(vaddr: Option<VirtAddr>);

    fn root_paddr(&self) -> PhysAddr;

    fn get_entry(&mut self, vaddr: VirtAddr) -> AcoreResult<&mut dyn PageTableEntry>;

    fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: MMUFlags) -> AcoreResult {
        let entry = self.get_entry(vaddr)?;
        entry.set_addr(paddr);
        entry.set_flags(flags);
        Ok(())
    }

    fn unmap(&mut self, vaddr: VirtAddr) -> AcoreResult {
        self.get_entry(vaddr)?.clear();
        Ok(())
    }

    fn protect(&mut self, vaddr: VirtAddr, flags: MMUFlags) -> AcoreResult {
        self.get_entry(vaddr)?.set_flags(flags);
        Ok(())
    }

    fn query(&mut self, vaddr: VirtAddr) -> AcoreResult<PhysAddr> {
        Ok(self.get_entry(vaddr)?.addr())
    }

    fn current() -> ManuallyDrop<Self> {
        unsafe { Self::from_root(Self::current_root_paddr()) }
    }

    /// # Safety
    ///
    /// This function is unsafe because it switches the virtual address space.
    unsafe fn set_current(&self) {
        let old_root = Self::current_root_paddr();
        let new_root = self.root_paddr();
        debug!("switch table {:#x?} -> {:#x?}", old_root, new_root);
        if new_root != old_root {
            Self::set_current_root_paddr(new_root);
            Self::flush_tlb(None);
        }
    }
}

pub(super) struct VmMapper<PT: PageTable> {
    pub(super) pgtable: PT,
}

impl<PT: PageTable> VmMapper<PT> {
    pub fn map_area(&mut self, vma: &VmArea, target: PhysAddr) {
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

    pub fn unmap_area(&mut self, vma: &VmArea) {
        trace!("destory mapping: {:#x?}", vma);
        for vaddr in (vma.start..vma.end).step_by(PAGE_SIZE) {
            self.pgtable
                .unmap(vaddr)
                .map_err(|e| panic!("failed to unmap VA: {:#x?}, {:?}", vaddr, e))
                .unwrap()
        }
    }
}
