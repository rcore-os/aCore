use alloc::vec::Vec;
use core::ops::Range;

use crate::memory::PAGE_SIZE;

pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
pub const KERNEL_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
pub const PHYS_MEMORY_OFFSET: usize = 0x8000_0000;
pub const PHYS_MEMORY_END: usize = 0x8800_0000;

pub type FrameAlloc = bitmap_allocator::BitAlloc1M;

pub fn get_phys_memory_regions() -> Vec<Range<usize>> {
    extern "C" {
        fn kernel_end();
    }
    let start = kernel_end as usize - KERNEL_OFFSET + PAGE_SIZE;
    let end = PHYS_MEMORY_END;
    vec![start..end]
}
