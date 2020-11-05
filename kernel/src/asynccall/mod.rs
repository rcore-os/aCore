mod structs;

use alloc::sync::Arc;

use spin::Mutex;

use crate::error::AcoreResult;
use crate::memory::{
    addr::{is_aligned, virt_to_phys},
    areas::{PmAreaFixed, VmArea},
    MMUFlags, PAGE_SIZE,
};
use crate::task::Thread;

pub use structs::AsyncCallBuffer;

#[repr(C)]
#[derive(Debug)]
pub struct AsyncCallInfo {
    user_buf_ptr: usize,
    buf_size: usize,
}

pub fn setup_async_call(
    thread: &Thread,
    _arg0: usize,
    _arg1: usize,
    _flags: u64,
) -> AcoreResult<AsyncCallInfo> {
    let buf_size = core::mem::size_of::<AsyncCallBuffer>();
    let buf_ptr = thread
        .owned_res
        .lock()
        .alloc_async_call_buffer()?
        .unwrap()
        .as_ptr::<u8>();
    let start_paddr = virt_to_phys(buf_ptr as usize);
    let end_paddr = start_paddr + buf_size;
    debug_assert!(is_aligned(start_paddr));

    let mut vm = thread.vm.lock();
    let pma = PmAreaFixed::new(start_paddr, end_paddr)?;
    let user_buf_ptr = vm.find_free_area(PAGE_SIZE, buf_size)?;
    let vma = VmArea::new(
        user_buf_ptr,
        user_buf_ptr + buf_size,
        MMUFlags::READ | MMUFlags::WRITE | MMUFlags::USER,
        Arc::new(Mutex::new(pma)),
        "async_call_buffer",
    )?;
    vm.push(vma)?;
    Ok(AsyncCallInfo {
        user_buf_ptr,
        buf_size,
    })
}

pub fn init() {
    info!("async call init end.");
}
