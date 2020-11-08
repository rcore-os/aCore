mod fs;
mod structs;

use alloc::{boxed::Box, sync::Arc};
use core::convert::TryFrom;

use spin::Mutex;

use crate::arch::cpu;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{is_aligned, virt_to_phys},
    areas::{PmAreaFixed, VmArea},
    MMUFlags, PAGE_SIZE,
};
use crate::sched::{yield_now, Executor};
use crate::task::Thread;
use structs::{AsyncCallType, CompleteRingEntry, RequestRingEntry};

pub use structs::{AsyncCallBuffer, AsyncCallInfoUser};

lazy_static! {
    static ref ASYNC_CALL_EXECUTOR: Executor = Executor::default();
}

pub struct AsyncCall {
    thread: Arc<Thread>,
}

type AsyncCallResult = AcoreResult<usize>;

impl AsyncCall {
    pub fn new(thread: Arc<Thread>) -> Self {
        Self { thread }
    }

    pub fn setup(
        thread: &Arc<Thread>,
        req_capacity: usize,
        comp_capacity: usize,
    ) -> AcoreResult<AsyncCallInfoUser> {
        // create shared memory
        if thread.owned_res.async_buf.lock().is_some() {
            return Err(AcoreError::AlreadyExists);
        }
        let buf = AsyncCallBuffer::new(req_capacity, comp_capacity)?;
        let buf_size = buf.size();
        let start_paddr = virt_to_phys(buf.as_ptr::<u8>() as usize);
        let end_paddr = start_paddr + buf_size;
        debug_assert!(is_aligned(start_paddr));

        // push to user's MemorySet
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
        let info = buf.fill_user_info(user_buf_ptr);
        *thread.owned_res.async_buf.lock() = Some(buf);

        // spawn async call polling coroutine and notify the I/O CPU
        let ac = Self::new(thread.clone());
        ASYNC_CALL_EXECUTOR.spawn(Box::pin(async move { ac.polling().await }));
        cpu::send_ipi(crate::config::IO_CPU_ID);

        Ok(info)
    }

    async fn do_async_call(&self, req: &RequestRingEntry) -> AsyncCallResult {
        let ac_type = match AsyncCallType::try_from(req.opcode) {
            Ok(t) => t,
            Err(_) => {
                error!("invalid async call number: {}", req.opcode);
                return Err(AcoreError::InvalidArgs);
            }
        };
        debug!("AsyncCall: {:?} => {:x?}", ac_type, req);
        let fd = req.fd as usize;
        let offset = req.offset as usize;
        let user_buf_addr = req.user_buf_addr as usize;
        let buf_size = req.buf_size as usize;
        let ret = match ac_type {
            AsyncCallType::Nop => Ok(0),
            AsyncCallType::Read => {
                self.async_read(fd, user_buf_addr.into(), buf_size, offset)
                    .await
            }
            AsyncCallType::Write => {
                self.async_write(fd, user_buf_addr.into(), buf_size, offset)
                    .await
            }
            _ => {
                warn!("asynca call unimplemented: {:?}", ac_type);
                Err(AcoreError::NotSupported)
            }
        };
        if ret.is_err() {
            warn!("AsyncCall: {:?} <= {:?}", ac_type, ret);
        } else {
            info!("AsyncCall: {:?} <= {:?}", ac_type, ret);
        }
        ret
    }

    async fn polling(&self) {
        info!("start async call polling for thread {}...", self.thread.id);
        while !self.thread.is_exited() {
            let buf_lock = self.thread.owned_res.async_buf.lock();
            let buf = match buf_lock.as_ref() {
                Some(b) => b,
                None => break,
            };
            debug!("thread {}: {:#x?}", self.thread.id, buf.as_raw());

            let mut cached_req_head = *buf.req_ring_head();
            let mut cached_comp_tail = *buf.comp_ring_tail();
            while cached_req_head < buf.req_ring_tail() {
                if self.thread.is_exited() {
                    break;
                }
                let req = buf.req_entry_at(cached_req_head);
                let res = self.do_async_call(&req).await;
                while cached_comp_tail - buf.comp_ring_head() == buf.comp_capacity {
                    // TODO: barriers
                    yield_now().await;
                }
                *buf.comp_entry_at(cached_comp_tail) = CompleteRingEntry::new(req.user_data, res);
                cached_comp_tail += 1;
                *buf.comp_ring_tail() = cached_comp_tail;
                cached_req_head += 1;
            }
            *buf.req_ring_head() = cached_req_head;
            yield_now().await;
        }
        info!("async call polling for thread {} is done.", self.thread.id);
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
