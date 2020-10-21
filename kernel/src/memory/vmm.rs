//! Virtual memory management.

#![allow(dead_code)]

use alloc::collections::{btree_map::Entry, BTreeMap};
use core::fmt::{Debug, Formatter, Result};
use spin::Mutex;

use lazy_static::lazy_static;

use super::addr::{align_down, align_up, VirtAddr};
use super::paging::{MMUFlags, PageTable, VmMapper};
use crate::arch::memory::{ArchPageTable, PHYS_VIRT_OFFSET};
use crate::error::{AcoreError, AcoreResult};

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
pub struct MemorySet<PT: PageTable = ArchPageTable> {
    areas: BTreeMap<usize, VmArea>,
    mapper: VmMapper<PT>,
}

impl VmArea {
    pub fn new(
        start: VirtAddr,
        end: VirtAddr,
        flags: MMUFlags,
        name: &'static str,
    ) -> AcoreResult<Self> {
        if start >= end {
            warn!("invalid memory region: [{:#x?}, {:#x?})", start, end);
            return Err(AcoreError::InvalidArgs);
        }
        Ok(Self {
            start: align_down(start),
            end: align_up(end),
            flags,
            name,
        })
    }

    /// Test whether a virtual address is contained in the memory area
    fn contains(&self, vaddr: VirtAddr) -> bool {
        self.start <= vaddr && vaddr < self.end
    }

    /// Test whether this area is (page) overlap with region [`start`, `end`)
    fn is_overlap_with(&self, start: VirtAddr, end: VirtAddr) -> bool {
        let p0 = self.start;
        let p1 = self.end;
        let p2 = align_down(start);
        let p3 = align_up(end);
        !(p1 <= p2 || p0 >= p3)
    }
}

impl<PT: PageTable> MemorySet<PT> {
    pub fn new() -> Self {
        Self {
            mapper: VmMapper { pgtable: PT::new() },
            areas: BTreeMap::new(),
        }
    }

    /// Find a free area with hint address `addr_hint` and length `len`.
    /// Return the start address of found free area.
    /// Used for mmap.
    pub fn find_free_area(&self, addr_hint: VirtAddr, len: usize) -> AcoreResult<VirtAddr> {
        // brute force:
        // try each area's end address as the start
        core::iter::once(align_up(addr_hint))
            .chain(self.areas.iter().map(|(_, area)| area.end))
            .find(|&addr| self.test_free_area(addr, addr + len))
            .ok_or(AcoreError::NoMemory)
    }

    /// Test whether [`start`, `end`) does not overlap with any existing areas.
    fn test_free_area(&self, start: VirtAddr, end: VirtAddr) -> bool {
        if let Some((_, before)) = self.areas.range(..start).last() {
            if before.is_overlap_with(start, end) {
                return false;
            }
        }
        if let Some((_, after)) = self.areas.range(start..).next() {
            if after.is_overlap_with(start, end) {
                return false;
            }
        }
        true
    }

    /// Add a VMA to this set.
    pub fn push(&mut self, vma: VmArea) -> AcoreResult {
        if !self.test_free_area(vma.start, vma.end) {
            warn!("VMA overlap: {:#x?}\n{:#x?}", vma, self);
            return Err(AcoreError::InvalidArgs);
        }
        self.mapper.map_area(&vma, vma.start - PHYS_VIRT_OFFSET);
        self.areas.insert(vma.start, vma);
        Ok(())
    }

    /// Remove the area `[start_addr, end_addr)` from `MemorySet`.
    pub fn pop(&mut self, start: VirtAddr, end: VirtAddr) -> AcoreResult {
        if start >= end {
            warn!("invalid memory region: [{:#x?}, {:#x?})", start, end);
            return Err(AcoreError::InvalidArgs);
        }
        let start = align_down(start);
        let end = align_up(end);
        if let Entry::Occupied(e) = self.areas.entry(start) {
            if e.get().end == end {
                self.mapper.unmap_area(e.get());
                e.remove();
                return Ok(());
            }
        }
        if self.test_free_area(start, end) {
            warn!(
                "no matched VMA found for memory region: [{:#x?}, {:#x?})",
                start, end
            );
            Err(AcoreError::InvalidArgs)
        } else {
            warn!(
                "partially unmap memory region [{:#x?}, {:#x?}) is not supported",
                start, end
            );
            Err(AcoreError::NotSupported)
        }
    }

    /// Clear and unmap all areas.
    pub fn clear(&mut self) {
        for area in self.areas.values() {
            self.mapper.unmap_area(area);
        }
        self.areas.clear();
    }

    /// Activate the associated page table.
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
            .field("areas", &self.areas.values())
            .field("page_table_root", &self.mapper.pgtable.root_paddr())
            .finish()
    }
}

lazy_static! {
    pub static ref KERNEL_MEMORY_SET: Mutex<MemorySet> = Mutex::new(MemorySet::new());
}

/// Re-build a fine-grained kernel page table, push memory segments to kernel memory set.
fn init_kernel_memory_set(ms: &mut MemorySet) -> AcoreResult {
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

    ms.push(VmArea::new(
        stext as usize,
        etext as usize,
        MMUFlags::READ | MMUFlags::EXECUTE,
        "ktext",
    )?)?;
    ms.push(VmArea::new(
        sdata as usize,
        edata as usize,
        MMUFlags::READ | MMUFlags::WRITE,
        "kdata",
    )?)?;
    ms.push(VmArea::new(
        srodata as usize,
        erodata as usize,
        MMUFlags::READ,
        "krodata",
    )?)?;
    ms.push(VmArea::new(
        sbss as usize,
        ebss as usize,
        MMUFlags::READ | MMUFlags::WRITE,
        "kbss",
    )?)?;
    ms.push(VmArea::new(
        boot_stack as usize,
        boot_stack_top as usize,
        MMUFlags::READ | MMUFlags::WRITE,
        "kstack",
    )?)?;
    for region in crate::arch::memory::get_phys_memory_regions() {
        ms.push(VmArea::new(
            region.start + PHYS_VIRT_OFFSET,
            region.end + PHYS_VIRT_OFFSET,
            MMUFlags::READ | MMUFlags::WRITE,
            "physical_memory",
        )?)?;
    }
    crate::arch::memory::create_mapping(ms)?;
    Ok(())
}

/// Initialize the kernel memory set and page table only on the primary CPU.
pub fn init() {
    let mut ms = KERNEL_MEMORY_SET.lock();
    init_kernel_memory_set(&mut ms).unwrap();
    unsafe { ms.activate() };
    info!("kernel memory set init end:\n{:#x?}", ms);
}

/// Activate the kernel page table on the secondary CPUs.
pub fn secondary_init() {
    unsafe { KERNEL_MEMORY_SET.lock().activate() };
}
