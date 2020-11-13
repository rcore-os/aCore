mod fs;
mod structs;

use alloc::{boxed::Box, sync::Arc};
use core::convert::TryFrom;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use spin::Mutex;

use crate::arch::cpu;
use crate::config::IO_CPU_ID;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{
    addr::{is_aligned, virt_to_phys},
    areas::{PmAreaFixed, VmArea},
    MMUFlags, PAGE_SIZE,
};
use crate::sched::yield_now;
use crate::task::{PerCpu, Thread};
use structs::{AsyncCallType, CompletionRingEntry, RequestRingEntry};

pub use structs::{AsyncCallBuffer, AsyncCallInfoUser};

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
        spawn_polling(thread);

        Ok(info)
    }

    async fn do_async_call(&self, req: &RequestRingEntry) -> AsyncCallResult {
        if self.thread.is_exited() {
            return Err(AcoreError::BadState);
        }
        let ac_type = match AsyncCallType::try_from(req.opcode) {
            Ok(t) => t,
            Err(_) => {
                error!("invalid async call number: {}", req.opcode);
                return Err(AcoreError::InvalidArgs);
            }
        };
        debug!("AsyncCall: {:?} => {:x?}", ac_type, req);
        let fd = req.fd as usize;
        let flags = req.flags as usize;
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
            AsyncCallType::Open => self.async_open(user_buf_addr.into(), flags).await,
            AsyncCallType::Close => self.async_close(fd).await,
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

    async fn polling_once(&self) -> AcoreResult {
        let buf_lock = self.thread.owned_res.async_buf.lock();
        let buf = match buf_lock.as_ref() {
            Some(b) => b,
            None => return Err(AcoreError::BadState),
        };
        debug!("thread {}: {:#x?}", self.thread.id, buf.as_raw());

        let mut cached_req_head = buf.read_req_ring_head();
        let mut cached_comp_tail = buf.read_comp_ring_tail();
        let req_count = buf.request_count(cached_req_head)?;
        // TODO: limit requests count or time for one thread
        for _ in 0..req_count {
            if self.thread.is_exited() {
                break;
            }
            let req_entry = buf.req_entry_at(cached_req_head);
            let res = self.do_async_call(&req_entry).await;
            while buf.completion_count(cached_comp_tail)? == buf.comp_capacity {
                yield_now().await;
            }
            *buf.comp_entry_at(cached_comp_tail) =
                CompletionRingEntry::new(req_entry.user_data, res);
            cached_comp_tail += 1;
            buf.write_comp_ring_tail(cached_comp_tail);
            cached_req_head += 1;
        }
        buf.write_req_ring_head(cached_req_head);
        Ok(())
    }

    async fn polling(&self) {
        info!("start async call polling for thread {}...", self.thread.id);
        while !self.thread.is_exited() {
            let res = self.polling_once().await;
            if let Err(e) = res {
                self.thread.exit(e as usize);
                break;
            }
            yield_now().await;
        }
        info!("async call polling for thread {} is done.", self.thread.id);
    }
}

type AsyncCallFuture = dyn Future<Output = ()> + Send;
type AsyncCallFuturePinned = Pin<Box<AsyncCallFuture>>;

struct AsyncCallSwitchFuture {
    thread: Arc<Thread>,
    future: AsyncCallFuturePinned,
}

impl AsyncCallSwitchFuture {
    fn new(thread: Arc<Thread>, future: AsyncCallFuturePinned) -> Self {
        Self { thread, future }
    }
}

impl Future for AsyncCallSwitchFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        PerCpu::set_current_thread(&self.thread);
        self.get_mut().future.as_mut().poll(cx)
    }
}

fn spawn_polling(thread: &Arc<Thread>) {
    let ac = AsyncCall::new(thread.clone());
    PerCpu::from_cpu_id(IO_CPU_ID).spawn(AsyncCallSwitchFuture::new(
        thread.clone(),
        Box::pin(async move { ac.polling().await }),
    ));
    cpu::send_ipi(IO_CPU_ID);
}

pub fn init() {
    info!("async call init end.");
}

pub fn run_forever() -> ! {
    loop {
        PerCpu::from_cpu_id(IO_CPU_ID).run_until_idle();
        info!("no async coroutines to run, waiting for interrupt...");
        cpu::wait_for_interrupt();
    }
}
