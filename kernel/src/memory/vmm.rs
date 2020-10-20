//! Virtual memory management.

use alloc::collections::LinkedList;
use core::fmt::{Debug, Formatter, Result};
use spin::Mutex;

use super::addr::{align_down, align_up, VirtAddr};
use super::paging::{Mapper, PageTable, PageTableFlags};
use crate::arch::memory::PHYS_VIRT_OFFSET;

/// A continuous virtual memory area with same flags.
#[derive(Debug)]
pub struct VmArea {
    pub(super) start: VirtAddr,
    pub(super) end: VirtAddr,
    pub(super) flags: PageTableFlags,
    name: &'static str,
}

/// A set of virtual memory areas with the associated page table.
pub struct MemorySet {
    areas: LinkedList<VmArea>,
    mapper: Mapper,
}

impl VmArea {
    pub fn new(start: VirtAddr, end: VirtAddr, flags: PageTableFlags, name: &'static str) -> Self {
        if start > end {
            panic!("invalid memory area [{:#x?}, {:#x?}]", start, end);
        }
        Self {
            start: align_down(start),
            end: align_up(end),
            flags,
            name,
        }
    }
}

impl MemorySet {
    pub fn new() -> Self {
        Self {
            mapper: Mapper::new(PageTable::new()),
            areas: LinkedList::new(),
        }
    }

    pub fn push(&mut self, vma: VmArea) {
        self.mapper.map(&vma, vma.start - PHYS_VIRT_OFFSET);
        self.areas.push_back(vma);
    }

    pub fn clear(&mut self) {
        for area in self.areas.iter() {
            self.mapper.unmap(area);
        }
        self.areas.clear();
    }

    pub fn page_table_ref(&self) -> &PageTable {
        &self.mapper.pgtable
    }
}

impl Drop for MemorySet {
    fn drop(&mut self) {
        self.clear()
    }
}

impl Debug for MemorySet {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("MemorySet")
            .field("areas", &self.areas)
            .field("page_table_root", &self.page_table_ref().root_paddr())
            .finish()
    }
}

pub static KERNEL_MEMORY_SET: Mutex<Option<MemorySet>> = Mutex::new(None);

/// Initialize the virtual memory management.
pub fn init() {
    extern "C" {
        fn stext();
        fn etext();
        fn sdata();
        fn edata();
        fn srodata();
        fn erodata();
        fn sbss();
        fn ebss();
        fn boot_stack();
        fn boot_stack_top();
    }
    let mut ms = MemorySet::new();
    ms.push(VmArea::new(
        stext as usize,
        etext as usize,
        PageTableFlags::VALID | PageTableFlags::READABLE | PageTableFlags::EXECUTABLE,
        "ktext",
    ));
    ms.push(VmArea::new(
        sdata as usize,
        edata as usize,
        PageTableFlags::VALID | PageTableFlags::READABLE | PageTableFlags::WRITABLE,
        "kdata",
    ));
    ms.push(VmArea::new(
        srodata as usize,
        erodata as usize,
        PageTableFlags::VALID | PageTableFlags::READABLE,
        "krodata",
    ));
    ms.push(VmArea::new(
        sbss as usize,
        ebss as usize,
        PageTableFlags::VALID | PageTableFlags::READABLE | PageTableFlags::WRITABLE,
        "kbss",
    ));
    ms.push(VmArea::new(
        boot_stack as usize,
        boot_stack_top as usize,
        PageTableFlags::VALID | PageTableFlags::READABLE | PageTableFlags::WRITABLE,
        "kstack",
    ));

    let regions = crate::arch::memory::get_phys_memory_regions();
    for region in regions {
        ms.push(VmArea::new(
            region.start + PHYS_VIRT_OFFSET,
            region.end + PHYS_VIRT_OFFSET,
            PageTableFlags::VALID | PageTableFlags::READABLE | PageTableFlags::WRITABLE,
            "physical_memory",
        ));
    }
    crate::arch::memory::create_mapping(&mut ms);

    unsafe { ms.page_table_ref().set_current() };

    info!("kernel memory set init end:\n{:#x?}", ms);
    *KERNEL_MEMORY_SET.lock() = Some(ms);
}

pub fn secondary_init() {
    if let Some(ms) = KERNEL_MEMORY_SET.lock().as_ref() {
        unsafe { ms.page_table_ref().set_current() }
    } else {
        panic!("KERNEL_MEMORY_SET not initialized")
    }
}
