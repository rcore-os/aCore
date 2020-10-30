//! Physical memory allocation.

use bitmap_allocator::BitAlloc;
use core::mem::ManuallyDrop;

use spin::Mutex;

use super::{addr::phys_to_virt, PhysAddr, PAGE_SIZE, PHYS_MEMORY_OFFSET};
use crate::arch::memory::FrameAlloc;
use crate::error::{AcoreError, AcoreResult};

static FRAME_ALLOCATOR: Mutex<FrameAlloc> = Mutex::new(FrameAlloc::DEFAULT);

fn phys_addr_to_frame_idx(addr: PhysAddr) -> usize {
    (addr - PHYS_MEMORY_OFFSET) / PAGE_SIZE
}

fn frame_idx_to_phys_addr(idx: usize) -> PhysAddr {
    idx * PAGE_SIZE + PHYS_MEMORY_OFFSET
}

/// # Safety
///
/// This function is unsafe because your need to deallocate manually.
unsafe fn alloc_frame() -> Option<PhysAddr> {
    let ret = FRAME_ALLOCATOR.lock().alloc().map(frame_idx_to_phys_addr);
    trace!("Allocate frame: {:x?}", ret);
    ret
}

/// # Safety
///
/// This function is unsafe because your need to deallocate manually.
unsafe fn alloc_frame_contiguous(frame_count: usize, align_log2: usize) -> Option<PhysAddr> {
    let ret = FRAME_ALLOCATOR
        .lock()
        .alloc_contiguous(frame_count, align_log2)
        .map(frame_idx_to_phys_addr);
    trace!(
        "Allocate {} frames with alignment {}: {:x?}",
        frame_count,
        1 << align_log2,
        ret
    );
    ret
}

/// # Safety
///
/// This function is unsafe because the frame must have been allocated.
unsafe fn dealloc_frame(target: PhysAddr) {
    trace!("Deallocate frame: {:x}", target);
    FRAME_ALLOCATOR
        .lock()
        .dealloc(phys_addr_to_frame_idx(target))
}

/// # Safety
///
/// This function is unsafe because the frames must have been allocated.
unsafe fn dealloc_frame_contiguous(target: PhysAddr, frame_count: usize) {
    trace!("Deallocate {} frames: {:x}", frame_count, target);
    let start_idx = phys_addr_to_frame_idx(target);
    let mut ba = FRAME_ALLOCATOR.lock();
    for i in start_idx..start_idx + frame_count {
        ba.dealloc(i)
    }
}

/// Initialize the frame alloactor.
pub(super) fn init() {
    let mut ba = FRAME_ALLOCATOR.lock();
    let regions = crate::arch::memory::get_phys_memory_regions();
    for region in regions {
        let frame_start = phys_addr_to_frame_idx(region.start);
        let frame_end = phys_addr_to_frame_idx(region.end - 1) + 1;
        assert!(frame_start < frame_end, "illegal range for frame allocator");
        ba.insert(frame_start..frame_end);
    }
    info!("frame allocator init end.");
}

/// A safe wrapper for physical frame allocation.
#[derive(Debug)]
pub struct Frame {
    start_paddr: PhysAddr,
    frame_count: usize,
}

impl Frame {
    /// Allocate one physical frame.
    pub fn new() -> AcoreResult<Self> {
        unsafe {
            alloc_frame()
                .map(|start_paddr| Self {
                    start_paddr,
                    frame_count: 1,
                })
                .ok_or(AcoreError::NoMemory)
        }
    }

    /// Allocate contiguous physical frames.
    pub fn new_contiguous(frame_count: usize, align_log2: usize) -> AcoreResult<Self> {
        unsafe {
            alloc_frame_contiguous(frame_count, align_log2)
                .map(|start_paddr| Self {
                    start_paddr,
                    frame_count,
                })
                .ok_or(AcoreError::NoMemory)
        }
    }

    /// Constructs a frame from a raw physical address without automatically calling the destructor.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the user must ensure that this is an available physical
    /// frame.
    pub unsafe fn from_paddr(start_paddr: PhysAddr) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Self {
            start_paddr,
            frame_count: 1,
        })
    }

    /// Get the start physical address of this frame.
    pub fn start_paddr(&self) -> PhysAddr {
        self.start_paddr
    }

    /// Get the total size (in bytes) of this frame.
    pub fn size(&self) -> usize {
        self.frame_count * PAGE_SIZE
    }

    /// convert to raw a pointer.
    pub fn as_ptr(&self) -> *const u8 {
        phys_to_virt(self.start_paddr) as *const u8
    }

    /// convert to a mutable raw pointer.
    pub fn as_mut_ptr(&self) -> *mut u8 {
        phys_to_virt(self.start_paddr) as *mut u8
    }

    /// Fill `self` with `byte`.
    pub fn fill(&mut self, byte: u8) {
        unsafe { core::ptr::write_bytes(self.as_mut_ptr(), byte, self.size()) }
    }

    /// Fill `self` with zero.
    pub fn zero(&mut self) {
        self.fill(0)
    }

    /// Forms a slice that can read data.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.size()) }
    }

    /// Forms a mutable slice that can write data.
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.size()) }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            if self.frame_count == 1 {
                dealloc_frame(self.start_paddr)
            } else {
                dealloc_frame_contiguous(self.start_paddr, self.frame_count)
            }
        }
    }
}
