#![allow(dead_code)]

use alloc::vec::Vec;
use core::mem::ManuallyDrop;

use riscv::asm::{sfence_vma, sfence_vma_all};
pub use riscv::paging::PageTableFlags;
use riscv::paging::{FrameAllocator, Mapper, PageTable as RvPageTable, PageTableEntry};
use riscv::register::satp;

use crate::arch::memory::PHYS_VIRT_OFFSET;
use crate::memory::{addr::phys_to_virt, Frame, PagingError, PagingResult, PhysAddr, VirtAddr};

mod rv {
    pub use riscv::addr::{Frame, Page, PhysAddr, VirtAddr};
}

#[cfg(target_arch = "riscv64")]
type TopLevelPageTable<'a> = riscv::paging::Rv39PageTable<'a>;

pub struct PageTable {
    inner: TopLevelPageTable<'static>,
    root: Frame,
    allocator: PageTableFrameAllocator,
}

impl PageTable {
    pub fn new() -> Self {
        let root = Frame::new().expect("failed to allocate root frame for page table");
        let table = unsafe { &mut *(phys_to_virt(root.start_paddr()) as *mut RvPageTable) };
        table.zero();
        Self {
            inner: TopLevelPageTable::new(table, PHYS_VIRT_OFFSET),
            root,
            allocator: PageTableFrameAllocator::new(),
        }
    }

    /// Constructs a multi-level page table from a physical address of the root page table.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the user must ensure that the page table indicated by the
    /// memory region starting from `root_paddr` must has the correct format.
    unsafe fn from_root(root_paddr: PhysAddr) -> ManuallyDrop<Self> {
        let table = &mut *(phys_to_virt(root_paddr) as *mut RvPageTable);
        ManuallyDrop::new(Self {
            inner: TopLevelPageTable::new(table, PHYS_VIRT_OFFSET),
            root: ManuallyDrop::into_inner(Frame::from_paddr(root_paddr)),
            allocator: PageTableFrameAllocator::new(),
        })
    }

    pub fn root_paddr(&self) -> PhysAddr {
        self.root.start_paddr()
    }

    pub fn current_root_paddr() -> PhysAddr {
        satp::read().ppn() << 12
    }

    pub fn current() -> ManuallyDrop<Self> {
        unsafe { Self::from_root(Self::current_root_paddr()) }
    }

    /// # Safety
    ///
    /// This function is unsafe because it switches the page table.
    pub unsafe fn set_current(&self) {
        let old_root = Self::current_root_paddr();
        let new_root = self.root_paddr();
        debug!("switch table {:#x?} -> {:#x?}", old_root, new_root);
        if new_root != old_root {
            satp::set(satp::Mode::Sv39, 0, new_root >> 12);
            Self::flush_tlb(None);
        }
    }

    pub fn flush_tlb(vaddr: Option<VirtAddr>) {
        unsafe {
            if let Some(vaddr) = vaddr {
                sfence_vma(0, vaddr)
            } else {
                sfence_vma_all()
            }
        }
    }

    pub fn get_entry(&mut self, vaddr: VirtAddr) -> PagingResult<&mut PageTableEntry> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner.ref_entry(page).map_err(|_| PagingError::NoEntry)
    }

    pub fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: PageTableFlags) -> PagingResult {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        let frame = rv::Frame::of_addr(rv::PhysAddr::new(paddr));
        self.inner
            .map_to(page, frame, flags, &mut self.allocator)
            .map_err(|_| PagingError::MapError)?
            .flush();
        Ok(())
    }

    pub fn unmap(&mut self, vaddr: VirtAddr) -> PagingResult {
        self.get_entry(vaddr)
            .map_err(|_| PagingError::UnmapError)?
            .set_unused();
        Ok(())
    }
}

struct PageTableFrameAllocator {
    frames: Vec<Frame>,
}

impl PageTableFrameAllocator {
    fn new() -> Self {
        Self { frames: Vec::new() }
    }
}

impl FrameAllocator for PageTableFrameAllocator {
    fn alloc(&mut self) -> Option<rv::Frame> {
        Frame::new().map(|f| {
            let ret = rv::Frame::of_addr(rv::PhysAddr::new(f.start_paddr()));
            self.frames.push(f);
            ret
        })
    }
}
