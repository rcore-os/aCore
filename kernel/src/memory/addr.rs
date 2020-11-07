//! Definition of phyical and virtual addresses.

use super::{PAGE_SIZE, PHYS_VIRT_OFFSET};

pub type VirtAddr = usize;
pub type PhysAddr = usize;

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
