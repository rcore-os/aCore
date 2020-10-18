use alloc::vec::Vec;
use core::ops::Range;

use crate::memory::{virt_to_phys, PAGE_SIZE};

pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
pub const MEMORY_OFFSET: usize = 0x8000_0000;
pub const MEMORY_END: usize = 0x8800_0000;

pub type FrameAlloc = bitmap_allocator::BitAlloc1M;

pub fn get_phys_memory_regions() -> Vec<Range<usize>> {
    extern "C" {
        fn kernel_end();
    }
    let start = virt_to_phys(kernel_end as usize) + PAGE_SIZE;
    let end = MEMORY_END;
    vec![start..end]
}
