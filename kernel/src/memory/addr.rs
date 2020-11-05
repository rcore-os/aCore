//! Definition of phyical and virtual addresses.

use core::ops::{Deref, DerefMut};

use super::{PAGE_SIZE, PHYS_VIRT_OFFSET};

pub type VirtAddr = usize;
pub type PhysAddr = usize;

#[repr(C, align(4096))]
pub struct PageAligned<T>(T);

impl<T> PageAligned<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Deref for PageAligned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for PageAligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    paddr + PHYS_VIRT_OFFSET
}

pub fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    vaddr - PHYS_VIRT_OFFSET
}

pub fn align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

pub fn align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub fn is_aligned(addr: usize) -> bool {
    page_offset(addr) == 0
}

pub fn page_count(size: usize) -> usize {
    align_up(size) / PAGE_SIZE
}

pub fn page_offset(addr: usize) -> usize {
    addr & (PAGE_SIZE - 1)
}
