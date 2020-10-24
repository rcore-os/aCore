use alloc::vec::Vec;
use core::{convert::From, mem::ManuallyDrop};

use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::paging::{
    FrameAllocator, Mapper, PageTable as PT, PageTableEntry as PTE, PageTableFlags as PTF,
};
use riscv::register::satp;

use crate::arch::memory::PHYS_VIRT_OFFSET;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{addr::phys_to_virt, Frame, PhysAddr, VirtAddr};
use crate::memory::{MMUFlags, PageTable, PageTableEntry};

mod rv {
    pub use riscv::addr::{Frame, Page, PhysAddr, VirtAddr};
    pub use riscv::paging::{FlagUpdateError, MapToError, UnmapError};
}

#[cfg(target_arch = "riscv64")]
type TopLevelPageTable<'a> = riscv::paging::Rv39PageTable<'a>;

pub struct RvPageTable {
    inner: TopLevelPageTable<'static>,
    root: Frame,
    allocator: PageTableFrameAllocator,
}

impl From<MMUFlags> for PTF {
    fn from(f: MMUFlags) -> Self {
        let mut ret = PTF::VALID;
        if f.contains(MMUFlags::READ) {
            ret |= PTF::READABLE;
        }
        if f.contains(MMUFlags::WRITE) {
            ret |= PTF::WRITABLE;
        }
        if f.contains(MMUFlags::EXECUTE) {
            ret |= PTF::EXECUTABLE;
        }
        if f.contains(MMUFlags::USER) {
            ret |= PTF::USER;
        }
        ret
    }
}

impl From<PTF> for MMUFlags {
    fn from(f: PTF) -> Self {
        let mut ret = MMUFlags::empty();
        if f.contains(PTF::READABLE) {
            ret |= MMUFlags::READ;
        }
        if f.contains(PTF::WRITABLE) {
            ret |= MMUFlags::WRITE;
        }
        if f.contains(PTF::EXECUTABLE) {
            ret |= MMUFlags::EXECUTE;
        }
        if f.contains(PTF::USER) {
            ret |= MMUFlags::USER;
        }
        ret
    }
}

impl PageTableEntry for PTE {
    fn addr(&self) -> PhysAddr {
        self.addr().as_usize()
    }
    fn flags(&self) -> MMUFlags {
        self.flags().into()
    }
    fn is_present(&self) -> bool {
        self.flags().contains(PTF::VALID)
    }
    fn set_addr(&mut self, paddr: PhysAddr) {
        let frame = rv::Frame::of_addr(rv::PhysAddr::new(paddr));
        self.set(frame, self.flags())
    }
    fn set_flags(&mut self, flags: MMUFlags) {
        self.set(self.frame(), flags.into())
    }
    fn clear(&mut self) {
        self.set_unused()
    }
}

impl From<rv::MapToError> for AcoreError {
    fn from(err: rv::MapToError) -> Self {
        match err {
            rv::MapToError::FrameAllocationFailed => AcoreError::NoMemory,
            rv::MapToError::PageAlreadyMapped => AcoreError::AlreadyExists,
            _ => AcoreError::BadState,
        }
    }
}

impl From<rv::UnmapError> for AcoreError {
    fn from(err: rv::UnmapError) -> Self {
        match err {
            rv::UnmapError::PageNotMapped => AcoreError::NotFound,
            _ => AcoreError::BadState,
        }
    }
}

impl From<rv::FlagUpdateError> for AcoreError {
    fn from(_: rv::FlagUpdateError) -> Self {
        AcoreError::NotFound
    }
}
impl PageTable for RvPageTable {
    fn new() -> Self {
        let root = Frame::new().expect("failed to allocate root frame for page table");
        let table = unsafe { &mut *(phys_to_virt(root.start_paddr()) as *mut PT) };
        table.zero();
        Self {
            inner: TopLevelPageTable::new(table, PHYS_VIRT_OFFSET),
            root,
            allocator: PageTableFrameAllocator::new(),
        }
    }

    unsafe fn from_root(root_paddr: PhysAddr) -> ManuallyDrop<Self> {
        let table = &mut *(phys_to_virt(root_paddr) as *mut PT);
        ManuallyDrop::new(Self {
            inner: TopLevelPageTable::new(table, PHYS_VIRT_OFFSET),
            root: ManuallyDrop::into_inner(Frame::from_paddr(root_paddr)),
            allocator: PageTableFrameAllocator::new(),
        })
    }

    fn current_root_paddr() -> PhysAddr {
        satp::read().ppn() << 12
    }

    unsafe fn set_current_root_paddr(root_paddr: PhysAddr) {
        satp::set(satp::Mode::Sv39, 0, root_paddr >> 12)
    }

    fn flush_tlb(vaddr: Option<VirtAddr>) {
        unsafe {
            if let Some(vaddr) = vaddr {
                sfence_vma(0, vaddr)
            } else {
                sfence_vma_all()
            }
        }
    }

    fn root_paddr(&self) -> PhysAddr {
        self.root.start_paddr()
    }

    fn get_entry(&mut self, vaddr: VirtAddr) -> AcoreResult<&mut dyn PageTableEntry> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        Ok(self.inner.ref_entry(page)?)
    }

    fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: MMUFlags) -> AcoreResult {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        let frame = rv::Frame::of_addr(rv::PhysAddr::new(paddr));
        self.inner
            .map_to(page, frame, flags.into(), &mut self.allocator)?
            .flush();
        Ok(())
    }

    fn unmap(&mut self, vaddr: VirtAddr) -> AcoreResult {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner.unmap(page)?.1.flush();
        Ok(())
    }

    fn protect(&mut self, vaddr: VirtAddr, flags: MMUFlags) -> AcoreResult {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner.update_flags(page, flags.into())?.flush();
        Ok(())
    }

    fn query(&mut self, vaddr: VirtAddr) -> AcoreResult<PhysAddr> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner
            .translate_page(page)
            .map(|f| f.start_address().as_usize())
            .ok_or(AcoreError::NotFound)
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
