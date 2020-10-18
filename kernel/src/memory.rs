#![allow(dead_code)]

use bitmap_allocator::BitAlloc;
use buddy_system_allocator::LockedHeap;
use spin::Mutex;

use crate::arch::memory::{FrameAlloc, KERNEL_HEAP_SIZE, MEMORY_OFFSET, PHYS_VIRT_OFFSET};

pub const PAGE_SIZE: usize = 0x1000;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::new();

static FRAME_ALLOCATOR: Mutex<FrameAlloc> = Mutex::new(FrameAlloc::DEFAULT);

fn phys_addr_to_frame_idx(addr: usize) -> usize {
    (addr - MEMORY_OFFSET) / PAGE_SIZE
}

fn frame_idx_to_phys_addr(idx: usize) -> usize {
    idx * PAGE_SIZE + MEMORY_OFFSET
}

fn init_heap() {
    const MACHINE_ALIGN: usize = core::mem::size_of::<usize>();
    const HEAP_BLOCK: usize = KERNEL_HEAP_SIZE / MACHINE_ALIGN;
    static mut HEAP: [usize; HEAP_BLOCK] = [0; HEAP_BLOCK];
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, HEAP_BLOCK * MACHINE_ALIGN);
    }
    info!("heap init end");
}

fn init_frame_allocator() {
    let mut ba = FRAME_ALLOCATOR.lock();
    let regions = crate::arch::memory::get_phys_memory_regions();
    for region in regions {
        let frame_start = phys_addr_to_frame_idx(region.start);
        let frame_end = phys_addr_to_frame_idx(region.end - 1) + 1;
        assert!(frame_start < frame_end, "illegal range for frame allocator");
        ba.insert(frame_start..frame_end);
    }
    info!("frame allocator init end");
}

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
    init_heap();
    init_frame_allocator();
}

pub fn phys_to_virt(paddr: usize) -> usize {
    paddr + PHYS_VIRT_OFFSET
}

pub fn virt_to_phys(vaddr: usize) -> usize {
    vaddr - PHYS_VIRT_OFFSET
}

pub fn alloc_frame() -> Option<usize> {
    FRAME_ALLOCATOR.lock().alloc().map(frame_idx_to_phys_addr)
}

pub fn dealloc_frame(target: usize) {
    FRAME_ALLOCATOR
        .lock()
        .dealloc(phys_addr_to_frame_idx(target))
}
