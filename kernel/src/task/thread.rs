use alloc::collections::BTreeMap;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
    fmt::{Debug, Formatter, Result},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use lazy_static::lazy_static;
use spin::Mutex;

use super::context::{handle_user_trap, ThreadContext};
use crate::arch::context::ArchThreadContext;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::addr::{virt_to_phys, VirtAddr};
use crate::memory::areas::{PmAreaDelay, VmArea};
use crate::memory::{
    MMUFlags, MemorySet, KERNEL_MEMORY_SET, PAGE_SIZE, USER_STACK_OFFSET, USER_STACK_SIZE,
};
use crate::sched::yield_now;
use crate::utils::IdAllocator;

type ThreadFuture = dyn Future<Output = AcoreResult> + Send;
type ThreadFuturePinned = Pin<Box<ThreadFuture>>;

#[derive(Debug, Default)]
struct ThreadState {
    need_sched: bool,
    exited: bool,
}

pub struct Thread<C: ThreadContext = ArchThreadContext> {
    pub id: usize,
    pub cpu: usize,
    pub is_user: bool,
    pub vm: Arc<Mutex<MemorySet>>,
    context: Mutex<Option<Box<C>>>,
    state: Mutex<ThreadState>,
    future: Mutex<ThreadFuturePinned>,
}

lazy_static! {
    #[repr(align(64))]
    static ref TID_ALLOCATOR: Mutex<IdAllocator> = Mutex::new(IdAllocator::new(1..65536));
    #[repr(align(64))]
    static ref THREAD_POOL: Mutex<BTreeMap<usize, Arc<Thread>>> = Mutex::new(BTreeMap::new());
    #[repr(align(64))]
    static ref ZOMBIES: Mutex<Vec<Arc<Thread>>> = Mutex::new(Vec::new());
}

impl Thread {
    fn new(is_user: bool) -> AcoreResult<Arc<Self>> {
        let vm = if is_user {
            Arc::new(Mutex::new(MemorySet::new_user()))
        } else {
            KERNEL_MEMORY_SET.clone()
        };
        let th = Arc::new(Self {
            id: TID_ALLOCATOR.lock().alloc()?,
            cpu: crate::arch::cpu::id(),
            is_user,
            vm,
            context: Mutex::new(None),
            state: Mutex::new(ThreadState::default()),
            future: Mutex::new(Box::pin(async { Ok(()) })),
        });
        THREAD_POOL.lock().insert(th.id, th.clone());
        Ok(th)
    }

    pub fn new_kernel(
        entry: impl Future<Output = AcoreResult> + Send + 'static,
    ) -> AcoreResult<Arc<Self>> {
        let th = Self::new(false)?;
        *th.future.lock() = Box::pin(entry);
        debug!("new kernel thread: {:#x?}", th);
        Ok(th)
    }

    pub fn new_user(entry: VirtAddr, arg: usize) -> AcoreResult<Arc<Self>> {
        let th = Self::new(true)?;
        let tmp = th.clone();
        *th.future.lock() = Box::pin(async move { tmp.run_user().await });
        th.init_user(entry, arg)?;
        debug!("new user thread: {:#x?}", th);
        Ok(th)
    }

    pub fn set_exited(&self) {
        self.state.lock().exited = true;
    }

    pub fn set_need_sched(&self) {
        self.state.lock().need_sched = true;
    }
}

impl Thread {
    fn init_user(self: &Arc<Self>, entry: VirtAddr, arg: usize) -> AcoreResult {
        let stack_bottom = USER_STACK_OFFSET;
        let stack_top = stack_bottom + USER_STACK_SIZE;
        let pma = PmAreaDelay::new(USER_STACK_SIZE)?;
        let stack = VmArea::new(
            stack_bottom,
            stack_top,
            MMUFlags::READ | MMUFlags::WRITE | MMUFlags::USER,
            Arc::new(Mutex::new(pma)),
            "stack",
        )?;
        self.vm.lock().push(stack)?;

        // test text segment
        let text_start_paddr = virt_to_phys(entry);
        let text_end_paddr = text_start_paddr + PAGE_SIZE;
        let text = VmArea::from_fixed_pma(
            text_start_paddr,
            text_end_paddr,
            0,
            MMUFlags::READ | MMUFlags::EXECUTE | MMUFlags::USER,
            "text",
        )?;
        self.vm.lock().push(text)?;

        let ctx = ArchThreadContext::new(virt_to_phys(entry), arg, stack_top, true);
        *self.context.lock() = Some(Box::from(ctx));
        Ok(())
    }

    fn tls_ptr(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self) as usize
    }

    fn exit(self: &Arc<Self>) {
        if self.is_user {
            self.vm.lock().clear(); // remove all user mappings
        }
        ZOMBIES.lock().push(self.clone()); // add to zombie thread list, it will finally drop in idle thread
        THREAD_POOL.lock().remove(&self.id);
    }

    async fn run_user(self: &Arc<Self>) -> AcoreResult {
        if !self.is_user {
            return Err(AcoreError::BadState);
        }
        loop {
            let mut ctx = self.context.lock().take().ok_or(AcoreError::BadState)?;
            let trap = ctx.run();
            handle_user_trap(trap, &mut ctx)?;
            ctx.end_trap(trap);
            *self.context.lock() = Some(ctx);

            let mut state = self.state.lock();
            if state.exited {
                break;
            }
            if state.need_sched {
                state.need_sched = false;
                yield_now().await?;
            }
        }
        Ok(())
    }
}

impl<C: ThreadContext> Drop for Thread<C> {
    fn drop(&mut self) {
        debug_assert!(self.id > 1); // idle thread cannot exit
        debug!("drop thread: {:#x?}", self);
        TID_ALLOCATOR.lock().dealloc(self.id);
    }
}

impl<C: ThreadContext> Debug for Thread<C> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut f = f.debug_struct("Thread");
        f.field("id", &self.id).field("cpu", &self.cpu);
        if self.is_user {
            f.field("vm", &self.vm);
        } else {
            f.field("vm", &format_args!("KERNEL_MEMORY_SET"));
        }
        f.field("context", &self.context)
            .field("state", &self.state)
            .field(
                "user_or_kernel",
                &if self.is_user {
                    format_args!("USER")
                } else {
                    format_args!("KERNEL")
                },
            );
        f.finish()
    }
}

pub(super) struct ThreadSwitchFuture {
    inner: Arc<Thread>,
}

impl ThreadSwitchFuture {
    pub fn new(thread: Arc<Thread>) -> Self {
        Self { inner: thread }
    }
}

impl Future for ThreadSwitchFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            crate::arch::context::write_tls(self.inner.tls_ptr());
            self.inner.vm.lock().activate();
        }
        self.inner.future.lock().as_mut().poll(cx).map(|res| {
            info!("thread {} exited with {:?}.", self.inner.id, res);
            self.inner.exit();
        })
    }
}

pub async fn idle() -> AcoreResult {
    loop {
        ZOMBIES.lock().clear(); // drop all zombie threads and deallocate their root page tables
        if THREAD_POOL.lock().len() == 1 {
            info!("no threads to run, waiting for interrupt...");
            crate::arch::cpu::wait_for_interrupt();
        }
        yield_now().await?;
    }
}
