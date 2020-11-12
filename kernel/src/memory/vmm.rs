//! Virtual memory management.

use alloc::collections::{btree_map::Entry, BTreeMap};
use alloc::sync::Arc;
use core::fmt::{Debug, Formatter, Result};

use spin::Mutex;

use super::addr::{align_down, align_up, virt_to_phys, VirtAddr};
use super::areas::VmArea;
use super::paging::{MMUFlags, PageTable};
use super::{KERNEL_STACK, PAGE_SIZE, USER_VIRT_ADDR_LIMIT};
use crate::arch::memory::ArchPageTable;
use crate::error::{AcoreError, AcoreResult};

/// A set of virtual memory areas with the associated page table.
pub struct MemorySet<PT: PageTable = ArchPageTable> {
    areas: BTreeMap<usize, VmArea>,
    pt: PT,
    is_user: bool,
}

impl<PT: PageTable> MemorySet<PT> {
    pub fn new_kernel() -> Self {
        Self {
            areas: BTreeMap::new(),
            pt: PT::new(),
            is_user: false,
        }
    }

    pub fn new_user() -> Self {
        let mut pt = PT::new();
        pt.map_kernel();
        Self {
            areas: BTreeMap::new(),
            pt,
            is_user: true,
        }
    }

    /// Find a free area with hint address `addr_hint` and length `len`.
    /// Return the start address of found free area.
    /// Used for mmap.
    pub fn find_free_area(&self, addr_hint: VirtAddr, len: usize) -> AcoreResult<VirtAddr> {
        // brute force:
        // try each area's end address as the start
        let addr = core::iter::once(align_up(addr_hint))
            .chain(self.areas.iter().map(|(_, area)| area.end))
            .find(|&addr| self.test_free_area(addr, addr + len))
            .unwrap();
        if addr >= USER_VIRT_ADDR_LIMIT {
            Err(AcoreError::NoMemory)
        } else {
            Ok(addr)
        }
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
        vma.map_area(&mut self.pt)?;
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
                e.get().unmap_area(&mut self.pt)?;
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

    /// Handle page fault.
    pub fn handle_page_fault(&mut self, vaddr: VirtAddr, access_flags: MMUFlags) -> AcoreResult {
        if let Some((_, area)) = self.areas.range(..=vaddr).last() {
            if area.contains(vaddr) {
                return area.handle_page_fault(vaddr - area.start, access_flags, &mut self.pt);
            }
        }
        warn!(
            "unhandled page fault @ {:#x?} with access {:?}",
            vaddr, access_flags
        );
        Err(AcoreError::Fault)
    }

    /// Clear and unmap all areas.
    pub fn clear(&mut self) {
        if !self.is_user {
            error!("cannot clear kernel memory set");
            return;
        }
        for area in self.areas.values() {
            area.unmap_area(&mut self.pt).unwrap();
        }
        self.areas.clear();
    }

    /// Activate the associated page table.
    pub unsafe fn activate(&self) {
        self.pt.set_current()
    }

    fn read_write(
        &self,
        start: VirtAddr,
        len: usize,
        access_flags: MMUFlags,
        mut op: impl FnMut(&VmArea, usize, usize, usize) -> AcoreResult,
    ) -> AcoreResult {
        let mut start = start;
        let mut len = len;
        let mut processed = 0;
        while len > 0 {
            if let Some((_, area)) = self.areas.range(..=start).last() {
                if area.end <= start {
                    return Err(AcoreError::Fault);
                }
                if !area.flags.contains(access_flags) {
                    return Err(AcoreError::AccessDenied);
                }
                let n = (area.end - start).min(len);
                op(area, start - area.start, n, processed)?;
                start += n;
                processed += n;
                len -= n;
            } else {
                return Err(AcoreError::Fault);
            }
        }
        Ok(())
    }

    pub fn read(
        &self,
        start: VirtAddr,
        len: usize,
        dst: &mut [u8],
        access_flags: MMUFlags,
    ) -> AcoreResult {
        self.read_write(start, len, access_flags, |area, offset, len, processed| {
            area.pma
                .lock()
                .read(offset, &mut dst[processed..processed + len])?;
            Ok(())
        })
    }

    pub fn write(
        &self,
        start: VirtAddr,
        len: usize,
        src: &[u8],
        access_flags: MMUFlags,
    ) -> AcoreResult {
        self.read_write(start, len, access_flags, |area, offset, len, processed| {
            area.pma
                .lock()
                .write(offset, &src[processed..processed + len])?;
            Ok(())
        })
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
            .field("page_table_root", &self.pt.root_paddr())
            .finish()
    }
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
    }

    use super::PHYS_VIRT_OFFSET;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(stext as usize),
        virt_to_phys(etext as usize),
        PHYS_VIRT_OFFSET,
        MMUFlags::READ | MMUFlags::EXECUTE,
        "ktext",
    )?)?;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(sdata as usize),
        virt_to_phys(edata as usize),
        PHYS_VIRT_OFFSET,
        MMUFlags::READ | MMUFlags::WRITE,
        "kdata",
    )?)?;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(srodata as usize),
        virt_to_phys(erodata as usize),
        PHYS_VIRT_OFFSET,
        MMUFlags::READ,
        "krodata",
    )?)?;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(sbss as usize),
        virt_to_phys(ebss as usize),
        PHYS_VIRT_OFFSET,
        MMUFlags::READ | MMUFlags::WRITE,
        "kbss",
    )?)?;
    for stack in &KERNEL_STACK {
        let per_cpu_stack_bottom = stack.as_ptr() as usize + PAGE_SIZE; // shadow page
        let per_cpu_stack_top = stack.as_ptr() as usize + stack.len();
        ms.push(VmArea::from_fixed_pma(
            virt_to_phys(per_cpu_stack_bottom),
            virt_to_phys(per_cpu_stack_top),
            PHYS_VIRT_OFFSET,
            MMUFlags::READ | MMUFlags::WRITE,
            "kstack",
        )?)?;
    }
    for region in crate::arch::memory::get_phys_memory_regions() {
        ms.push(VmArea::from_fixed_pma(
            region.start,
            region.end,
            PHYS_VIRT_OFFSET,
            MMUFlags::READ | MMUFlags::WRITE,
            "physical_memory",
        )?)?;
    }
    crate::arch::memory::create_mapping(ms)?;
    Ok(())
}

lazy_static! {
    #[repr(align(64))]
    pub static ref KERNEL_MEMORY_SET: Arc<Mutex<MemorySet>> = {
        let mut ms = MemorySet::new_kernel();
        init_kernel_memory_set(&mut ms).unwrap();
        info!("kernel memory set init end:\n{:#x?}", ms);
        Arc::new(Mutex::new(ms))
    };
}

/// Initialize the kernel memory set and activate kernel page table.
pub fn init() {
    unsafe { KERNEL_MEMORY_SET.lock().activate() };
}
