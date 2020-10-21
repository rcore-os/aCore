//! Physical and virtual memory management.

pub mod addr;
mod frame;
mod heap;
mod paging;
mod vmm;

pub use addr::{PhysAddr, VirtAddr};
pub use frame::Frame;
pub use paging::{MMUFlags, PageTable, PageTableEntry};
pub use vmm::MemorySet;

pub const PAGE_SIZE: usize = 0x1000;

pub fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let start = sbss as usize;
    let end = ebss as usize;
    let step = core::mem::size_of::<usize>();
    for i in (start..end).step_by(step) {
        unsafe { (i as *mut usize).write(0) };
    }
}

pub fn init() {
    heap::init();
    frame::init();
    vmm::init();
}

pub fn secondary_init() {
    vmm::secondary_init();
    info!("secondary CPU memory init end.");
}
