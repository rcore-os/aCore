mod fs;
mod structs;

use alloc::{boxed::Box, sync::Arc};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::cpu;
use crate::error::AcoreResult;
use crate::memory::{
    addr::{is_aligned, virt_to_phys},
    areas::{PmAreaFixed, VmArea},
    MMUFlags, PAGE_SIZE,
};
use crate::sched::{yield_now, Executor};
use crate::task::Thread;

pub use structs::AsyncCallBuffer;

lazy_static! {
    static ref ASYNC_CALL_EXECUTOR: Executor = Executor::default();
}

#[repr(C)]
#[derive(Debug)]
pub struct AsyncCallInfo {
    user_buf_ptr: usize,
    buf_size: usize,
}

pub struct AsyncCall {
    thread: Arc<Thread>,
}

impl AsyncCall {
    pub fn new(thread: Arc<Thread>) -> Self {
        Self { thread }
    }

    async fn polling(&self) {
        info!("start async call polling for thread {}...", self.thread.id);
        while !self.thread.is_exited() {
            self.read();
            yield_now().await.unwrap();
        }
        info!("async call polling for thread {} is done.", self.thread.id);
    }

    pub fn setup(
        thread: &Arc<Thread>,
        _arg0: usize,
        _arg1: usize,
        _flags: u64,
    ) -> AcoreResult<AsyncCallInfo> {
        // create shared memory
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

        // spawn async call polling coroutine and notify the I/O CPU
        let ac = Self::new(thread.clone());
        ASYNC_CALL_EXECUTOR.spawn(Box::pin(async move { ac.polling().await }));
        cpu::send_ipi(crate::config::IO_CPU_ID);

        Ok(AsyncCallInfo {
            user_buf_ptr,
            buf_size,
        })
    }
}

pub fn init() {
    info!("async call init end.");
}

pub fn run_forever() -> ! {
    loop {
        ASYNC_CALL_EXECUTOR.run_until_idle();
        info!("no async coroutines to run, waiting for interrupt...");
        cpu::wait_for_interrupt();
    }
}
