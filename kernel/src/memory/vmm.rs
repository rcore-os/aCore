//! Virtual memory management.

use alloc::collections::LinkedList;
use core::fmt::{Debug, Formatter, Result};
use spin::Mutex;

use lazy_static::lazy_static;

use super::addr::{align_down, align_up, VirtAddr};
use super::paging::{MMUFlags, PageTable, VmMapper};
use crate::arch::memory::{ArchPageTable, PHYS_VIRT_OFFSET};

/// A continuous virtual memory area with same flags.
/// The `start` and `end` address are page aligned.
#[derive(Debug)]
pub struct VmArea {
    pub(super) start: VirtAddr,
    pub(super) end: VirtAddr,
    pub(super) flags: MMUFlags,
    name: &'static str,
}

/// A set of virtual memory areas with the associated page table.
pub struct MemorySet<PT: PageTable> {
    areas: LinkedList<VmArea>,
    mapper: VmMapper<PT>,
}

impl VmArea {
    pub fn new(start: VirtAddr, end: VirtAddr, flags: MMUFlags, name: &'static str) -> Self {
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

impl<PT: PageTable> MemorySet<PT> {
    pub fn new() -> Self {
        Self {
            mapper: VmMapper { pgtable: PT::new() },
            areas: LinkedList::new(),
        }
    }

    pub fn push(&mut self, vma: VmArea) {
        self.mapper.map_area(&vma, vma.start - PHYS_VIRT_OFFSET);
        self.areas.push_back(vma);
    }

    pub fn clear(&mut self) {
        for area in self.areas.iter() {
            self.mapper.unmap_area(area);
        }
        self.areas.clear();
    }

    pub unsafe fn activate(&self) {
        self.mapper.pgtable.set_current()
    }
}

impl<PT: PageTable> Drop for MemorySet<PT> {
    fn drop(&mut self) {
        self.clear()
    }
}

impl<PT: PageTable> Debug for MemorySet<PT> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("MemorySet")
            .field("areas", &self.areas)
            .field("page_table_root", &self.mapper.pgtable.root_paddr())
            .finish()
    }
}

lazy_static! {
    pub static ref KERNEL_MEMORY_SET: Mutex<MemorySet<ArchPageTable>> =
        Mutex::new(MemorySet::new());
}

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

    let mut ms = KERNEL_MEMORY_SET.lock();
    ms.push(VmArea::new(
        stext as usize,
        etext as usize,
        MMUFlags::READ | MMUFlags::EXECUTE,
        "ktext",
    ));
    ms.push(VmArea::new(
        sdata as usize,
        edata as usize,
        MMUFlags::READ | MMUFlags::WRITE,
        "kdata",
    ));
    ms.push(VmArea::new(
        srodata as usize,
        erodata as usize,
        MMUFlags::READ,
        "krodata",
    ));
    ms.push(VmArea::new(
        sbss as usize,
        ebss as usize,
        MMUFlags::READ | MMUFlags::WRITE,
        "kbss",
    ));
    ms.push(VmArea::new(
        boot_stack as usize,
        boot_stack_top as usize,
        MMUFlags::READ | MMUFlags::WRITE,
        "kstack",
    ));

    for region in crate::arch::memory::get_phys_memory_regions() {
        ms.push(VmArea::new(
            region.start + PHYS_VIRT_OFFSET,
            region.end + PHYS_VIRT_OFFSET,
            MMUFlags::READ | MMUFlags::WRITE,
            "physical_memory",
        ));
    }
    crate::arch::memory::create_mapping(&mut ms);

    unsafe { ms.activate() };
    info!("kernel memory set init end:\n{:#x?}", ms);
}

pub fn secondary_init() {
    unsafe { KERNEL_MEMORY_SET.lock().activate() };
}
