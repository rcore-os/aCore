use alloc::vec::Vec;
use core::ops::Range;

use crate::error::AcoreResult;
use crate::memory::{addr, MemorySet};

pub use super::paging::RvPageTable as ArchPageTable;

pub mod consts {
    pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
    pub const USER_STACK_SIZE: usize = 0x10_0000; // 1 MB
    pub const USER_STACK_OFFSET: usize = 0x4000_0000 - USER_STACK_SIZE;

    pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
    pub const PHYS_MEMORY_OFFSET: usize = 0x8000_0000;
    pub const PHYS_MEMORY_END: usize = 0x8800_0000;
}

pub type FrameAlloc = bitmap_allocator::BitAlloc1M;

pub fn get_phys_memory_regions() -> Vec<Range<usize>> {
    extern "C" {
        fn kernel_end();
    }
    let start = addr::align_up(addr::virt_to_phys(kernel_end as usize));
    let end = consts::PHYS_MEMORY_END;
    vec![start..end]
}

pub fn create_mapping(_ms: &mut MemorySet) -> AcoreResult {
    Ok(())
}
