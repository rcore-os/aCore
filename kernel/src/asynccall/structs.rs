use core::intrinsics::{atomic_load_acq, atomic_store_rel};
use core::mem::size_of;

use numeric_enum_macro::numeric_enum;

use super::AsyncCallResult;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::cache::{alignup_cache_line, is_cache_line_aligned, AlignCacheLine};
use crate::memory::{addr::page_count, Frame, VirtAddr};
use crate::task::res_limit::MAX_ASYNC_CALL_ENTRY_NUM;

numeric_enum! {
#[repr(u8)]
#[derive(Debug, Eq, PartialEq)]
pub(super) enum AsyncCallType {
    Nop = 0,
    Read = 1,
    Write = 2,
    Open = 3,
    Close = 4,
    Unknown = 0xff,
}
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct RequestRingEntry {
    pub opcode: u8,
    _pad0: u8,
    _pad1: u16,
    pub fd: i32,
    pub offset: u64,
    pub user_buf_addr: u64,
    pub buf_size: u32,
    pub flags: u32,
    pub user_data: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
pub(super) struct CompletionRingEntry {
    user_data: u64,
    result: i32,
    _pad0: u32,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct Ring {
    head: AlignCacheLine<u32>,
    tail: AlignCacheLine<u32>,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct AsyncCallBufferLayout {
    req_ring: Ring,
    comp_ring: Ring,
    req_capacity: u32,
    req_capacity_mask: u32,
    comp_capacity: u32,
    comp_capacity_mask: u32,
    req_entries: AlignCacheLine<[RequestRingEntry; 0]>,
    comp_entries: AlignCacheLine<[CompletionRingEntry; 0]>,
}

#[repr(C)]
#[derive(Debug)]
struct RingOffsets {
    head: u32,
    tail: u32,
    capacity: u32,
    capacity_mask: u32,
    entries: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct AsyncCallInfoUser {
    user_buf_ptr: usize,
    buf_size: usize,
    req_off: RingOffsets,
    comp_off: RingOffsets,
}

#[repr(C)]
#[derive(Debug)]
pub struct AsyncCallBuffer {
    pub req_capacity: u32,
    pub comp_capacity: u32,
    req_capacity_mask: u32,
    comp_capacity_mask: u32,
    buf_size: usize,
    frame: Frame,
    frame_virt_addr: VirtAddr,
}

impl CompletionRingEntry {
    pub fn new(user_data: u64, res: AsyncCallResult) -> Self {
        Self {
            user_data,
            result: match res {
                Ok(code) => code as i32,
                Err(err) => err as i32,
            },
            ..Default::default()
        }
    }
}

impl AsyncCallBuffer {
    pub fn new(req_capacity: usize, comp_capacity: usize) -> AcoreResult<Self> {
        if req_capacity == 0 || req_capacity > MAX_ASYNC_CALL_ENTRY_NUM {
            return Err(AcoreError::InvalidArgs);
        }
        if comp_capacity == 0 || comp_capacity > MAX_ASYNC_CALL_ENTRY_NUM {
            return Err(AcoreError::InvalidArgs);
        }
        let req_capacity = req_capacity.next_power_of_two() as u32;
        let comp_capacity = comp_capacity.next_power_of_two() as u32;

        let req_entries_off = offset_of!(AsyncCallBufferLayout, req_entries);
        let comp_entries_off = alignup_cache_line(
            req_entries_off + size_of::<RequestRingEntry>() * req_capacity as usize,
        );
        let buf_size = comp_entries_off + size_of::<CompletionRingEntry>() * comp_capacity as usize;
        debug_assert!(is_cache_line_aligned(req_entries_off));
        debug_assert!(is_cache_line_aligned(comp_entries_off));

        let mut frame = Frame::new_contiguous(page_count(buf_size), 0)?;
        frame.zero();
        let frame_virt_addr = frame.as_ptr() as usize;

        let buf = unsafe { &mut *(frame.as_mut_ptr() as *mut AsyncCallBufferLayout) };
        buf.req_capacity = req_capacity;
        buf.comp_capacity = comp_capacity;
        buf.req_capacity_mask = req_capacity - 1;
        buf.comp_capacity_mask = comp_capacity - 1;

        Ok(Self {
            req_capacity,
            comp_capacity,
            req_capacity_mask: req_capacity - 1,
            comp_capacity_mask: comp_capacity - 1,
            buf_size,
            frame,
            frame_virt_addr,
        })
    }

    pub fn size(&self) -> usize {
        self.buf_size
    }

    pub(super) fn fill_user_info(&self, user_buf_ptr: usize) -> AsyncCallInfoUser {
        let req_entries_off = offset_of!(AsyncCallBufferLayout, req_entries);
        let comp_entries_off = alignup_cache_line(
            req_entries_off + size_of::<RequestRingEntry>() * self.req_capacity as usize,
        );
        AsyncCallInfoUser {
            user_buf_ptr,
            buf_size: self.buf_size,
            req_off: RingOffsets {
                head: (offset_of!(AsyncCallBufferLayout, req_ring) + offset_of!(Ring, head)) as _,
                tail: (offset_of!(AsyncCallBufferLayout, req_ring) + offset_of!(Ring, tail)) as _,
                capacity: offset_of!(AsyncCallBufferLayout, req_capacity) as _,
                capacity_mask: offset_of!(AsyncCallBufferLayout, req_capacity_mask) as _,
                entries: req_entries_off as _,
            },
            comp_off: RingOffsets {
                head: (offset_of!(AsyncCallBufferLayout, comp_ring) + offset_of!(Ring, head)) as _,
                tail: (offset_of!(AsyncCallBufferLayout, comp_ring) + offset_of!(Ring, tail)) as _,
                capacity: offset_of!(AsyncCallBufferLayout, comp_capacity) as _,
                capacity_mask: offset_of!(AsyncCallBufferLayout, comp_capacity_mask) as _,
                entries: comp_entries_off as _,
            },
        }
    }

    pub(super) fn read_req_ring_head(&self) -> u32 {
        self.as_raw().req_ring.head.get()
    }

    pub(super) fn write_req_ring_head(&self, new_head: u32) {
        unsafe { atomic_store_rel(self.as_raw_mut().req_ring.head.as_mut() as _, new_head) }
    }

    pub(super) fn read_req_ring_tail(&self) -> u32 {
        unsafe { atomic_load_acq(self.as_raw().req_ring.tail.as_ref() as _) }
    }

    pub(super) fn request_count(&self, cached_req_ring_head: u32) -> AcoreResult<u32> {
        let n = self.read_req_ring_tail().wrapping_sub(cached_req_ring_head);
        if n <= self.req_capacity {
            Ok(n)
        } else {
            Err(AcoreError::BadState)
        }
    }

    pub(super) fn read_comp_ring_head(&self) -> u32 {
        unsafe { atomic_load_acq(self.as_raw().comp_ring.head.as_ref() as _) }
    }

    pub(super) fn read_comp_ring_tail(&self) -> u32 {
        self.as_raw().comp_ring.tail.get()
    }

    pub(super) fn write_comp_ring_tail(&self, new_tail: u32) {
        unsafe { atomic_store_rel(self.as_raw_mut().comp_ring.tail.as_mut() as _, new_tail) }
    }

    pub(super) fn completion_count(&self, cached_comp_ring_tail: u32) -> AcoreResult<u32> {
        let n = cached_comp_ring_tail.wrapping_sub(self.read_comp_ring_head());
        if n <= self.comp_capacity {
            Ok(n)
        } else {
            Err(AcoreError::BadState)
        }
    }

    pub(super) fn req_entry_at(&self, idx: u32) -> &RequestRingEntry {
        unsafe {
            let ptr = self
                .as_ptr::<u8>()
                .add(offset_of!(AsyncCallBufferLayout, req_entries))
                as *const RequestRingEntry;
            &*ptr.add((idx & self.req_capacity_mask) as usize)
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn comp_entry_at(&self, idx: u32) -> &mut CompletionRingEntry {
        let comp_entries_off = alignup_cache_line(
            offset_of!(AsyncCallBufferLayout, req_entries)
                + size_of::<RequestRingEntry>() * self.req_capacity as usize,
        );
        unsafe {
            let ptr = self.as_mut_ptr::<u8>().add(comp_entries_off) as *mut CompletionRingEntry;
            &mut *ptr.add((idx & self.comp_capacity_mask) as usize)
        }
    }

    pub(super) fn as_ptr<T>(&self) -> *const T {
        self.frame_virt_addr as _
    }

    fn as_mut_ptr<T>(&self) -> *mut T {
        self.frame_virt_addr as _
    }

    pub(super) fn as_raw(&self) -> &AsyncCallBufferLayout {
        unsafe { &*self.as_ptr::<AsyncCallBufferLayout>() }
    }

    #[allow(clippy::mut_from_ref)]
    fn as_raw_mut(&self) -> &mut AsyncCallBufferLayout {
        unsafe { &mut *self.as_mut_ptr::<AsyncCallBufferLayout>() }
    }
}
