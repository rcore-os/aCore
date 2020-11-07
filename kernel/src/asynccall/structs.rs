use core::mem::size_of;
use core::slice;

use numeric_enum_macro::numeric_enum;

use super::AsyncCallResult;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{addr::page_count, Frame};

const MAX_ENTRY_COUNT: usize = 32768;

numeric_enum! {
#[repr(u8)]
#[derive(Debug, Eq, PartialEq)]
pub(super) enum AsyncCallType {
    Nop = 0,
    Read = 1,
    Write = 2,
    Unknown = 3,
}
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct RequestRingEntry {
    pub opcode: u8,
    pub flags: u8,
    _pad0: u16,
    pub fd: i32,
    pub offset: u64,
    pub user_buf_addr: u64,
    pub buf_size: u32,
    _pad1: u32,
    pub user_data: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
pub(super) struct CompleteRingEntry {
    user_data: u64,
    result: i32,
    _pad0: u32,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct Ring {
    head: usize,
    tail: usize,
    capacity: usize,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct AsyncCallBufferLayout {
    req_ring: Ring,
    comp_ring: Ring,
    req_entries: [RequestRingEntry; 0],
    comp_entries: [CompleteRingEntry; 0],
}

#[repr(C)]
#[derive(Debug)]
struct RingOffsets {
    head: u32,
    tail: u32,
    capacity: u32,
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
    pub req_capacity: usize,
    pub comp_capacity: usize,
    buf_size: usize,
    frame: Frame,
}

impl CompleteRingEntry {
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
        if req_capacity == 0 || req_capacity > MAX_ENTRY_COUNT {
            return Err(AcoreError::InvalidArgs);
        }
        if comp_capacity == 0 || comp_capacity > MAX_ENTRY_COUNT {
            return Err(AcoreError::InvalidArgs);
        }
        let req_capacity = req_capacity.next_power_of_two();
        let comp_capacity = comp_capacity.next_power_of_two();
        let buf_size = size_of::<AsyncCallBufferLayout>()
            + size_of::<RequestRingEntry>() * req_capacity
            + size_of::<CompleteRingEntry>() * comp_capacity;

        let mut frame = Frame::new_contiguous(page_count(buf_size), 0)?;
        frame.zero();

        let buf = unsafe { &mut *(frame.as_mut_ptr() as *mut AsyncCallBufferLayout) };
        buf.req_ring.capacity = req_capacity;
        buf.comp_ring.capacity = comp_capacity;

        Ok(Self {
            req_capacity,
            comp_capacity,
            buf_size,
            frame,
        })
    }

    pub fn size(&self) -> usize {
        self.buf_size
    }

    pub(super) fn fill_user_info(&self, user_buf_ptr: usize) -> AsyncCallInfoUser {
        AsyncCallInfoUser {
            user_buf_ptr,
            buf_size: self.buf_size,
            req_off: RingOffsets {
                head: (offset_of!(AsyncCallBufferLayout, req_ring) + offset_of!(Ring, head)) as _,
                tail: (offset_of!(AsyncCallBufferLayout, req_ring) + offset_of!(Ring, tail)) as _,
                capacity: (offset_of!(AsyncCallBufferLayout, req_ring) + offset_of!(Ring, capacity))
                    as _,
                entries: offset_of!(AsyncCallBufferLayout, req_entries) as _,
            },
            comp_off: RingOffsets {
                head: (offset_of!(AsyncCallBufferLayout, comp_ring) + offset_of!(Ring, head)) as _,
                tail: (offset_of!(AsyncCallBufferLayout, comp_ring) + offset_of!(Ring, tail)) as _,
                capacity: (offset_of!(AsyncCallBufferLayout, comp_ring)
                    + offset_of!(Ring, capacity)) as _,
                entries: (offset_of!(AsyncCallBufferLayout, req_entries)
                    + size_of::<RequestRingEntry>() * self.req_capacity)
                    as _,
            },
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn req_ring_head(&self) -> &mut usize {
        &mut self.as_raw_mut().req_ring.head
    }

    pub(super) fn req_ring_tail(&self) -> usize {
        self.as_raw().req_ring.tail
    }

    pub(super) fn comp_ring_head(&self) -> usize {
        self.as_raw().req_ring.head
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn comp_ring_tail(&self) -> &mut usize {
        &mut self.as_raw_mut().comp_ring.tail
    }

    pub(super) fn req_entry_at(&self, idx: usize) -> &RequestRingEntry {
        unsafe {
            let ptr = self
                .as_ptr::<u8>()
                .add(offset_of!(AsyncCallBufferLayout, req_entries))
                as *const RequestRingEntry;
            &slice::from_raw_parts(ptr, self.req_capacity)[idx]
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn comp_entry_at(&self, idx: usize) -> &mut CompleteRingEntry {
        unsafe {
            let ptr = self.as_mut_ptr::<u8>().add(
                offset_of!(AsyncCallBufferLayout, req_entries)
                    + size_of::<RequestRingEntry>() * self.req_capacity,
            ) as *mut CompleteRingEntry;
            &mut slice::from_raw_parts_mut(ptr, self.comp_capacity)[idx]
        }
    }

    pub(super) fn as_ptr<T>(&self) -> *const T {
        self.frame.as_ptr() as _
    }

    fn as_mut_ptr<T>(&self) -> *mut T {
        self.frame.as_mut_ptr() as _
    }

    pub(super) fn as_raw(&self) -> &AsyncCallBufferLayout {
        unsafe { &*self.as_ptr::<AsyncCallBufferLayout>() }
    }

    #[allow(clippy::mut_from_ref)]
    fn as_raw_mut(&self) -> &mut AsyncCallBufferLayout {
        unsafe { &mut *self.as_mut_ptr::<AsyncCallBufferLayout>() }
    }
}
