use alloc::collections::BTreeMap;
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::{
    fmt::{Debug, Formatter, Result},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use spin::Mutex;

use super::context::ThreadContext;
use super::resource::{OwnedResource, SharedResource};
use crate::arch::context::ArchThreadContext;
use crate::error::{AcoreError, AcoreResult};
use crate::fs::File;
use crate::memory::{MemorySet, KERNEL_MEMORY_SET};
use crate::sched::yield_now;
use crate::utils::{ElfLoader, IdAllocator};

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
    pub owned_res: Mutex<OwnedResource>,
    pub shared_res: Arc<Mutex<SharedResource>>,
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
    fn new(is_user: bool, vm: Arc<Mutex<MemorySet>>) -> AcoreResult<Arc<Self>> {
        let th = Arc::new(Self {
            id: TID_ALLOCATOR.lock().alloc()?,
            cpu: crate::arch::cpu::id(),
            is_user,
            vm,
            owned_res: Mutex::new(OwnedResource::default()),
            shared_res: Arc::new(Mutex::new(SharedResource::default())),
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
        let th = Self::new(false, KERNEL_MEMORY_SET.clone())?;
        *th.future.lock() = Box::pin(entry);
        debug!("new kernel thread: {:#x?}", th);
        Ok(th)
    }

    pub fn new_user(file: &File, args: Vec<String>) -> AcoreResult<Arc<Self>> {
        let loader = ElfLoader::new(file)?;
        let mut vm = MemorySet::new_user();
        let (entry_pointer, ustack_pointer) = loader.init_vm(&mut vm, args)?;

        let th = Self::new(true, Arc::new(Mutex::new(vm)))?;
        let tmp = th.clone();
        *th.future.lock() = Box::pin(async move { tmp.run_user().await });
        let ctx = ArchThreadContext::new(entry_pointer, ustack_pointer);
        *th.context.lock() = Some(Box::from(ctx));

        debug!("new user thread: {:#x?}", th);
        Ok(th)
    }

    pub fn is_exited(&self) -> bool {
        self.state.lock().exited
    }

    pub fn exit(&self, _code: usize) {
        self.state.lock().exited = true;
        if self.is_user {
            self.vm.lock().clear(); // remove all user mappings
        }
    }

    pub fn set_need_sched(&self) {
        self.state.lock().need_sched = true;
    }
}

impl Thread {
    fn tls_ptr(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self) as usize
    }

    async fn run_user(self: &Arc<Self>) -> AcoreResult {
        if !self.is_user {
            return Err(AcoreError::BadState);
        }
        loop {
            let mut ctx = self.context.lock().take().ok_or(AcoreError::BadState)?;
            let trap = ctx.run();
            self.handle_user_trap(trap, &mut ctx)?;
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

pub(super) struct ThreadSwitchFuture(Arc<Thread>);

impl ThreadSwitchFuture {
    pub fn new(thread: Arc<Thread>) -> Self {
        Self(thread)
    }
}

impl Future for ThreadSwitchFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            crate::arch::context::write_tls(self.0.tls_ptr());
            self.0.vm.lock().activate();
        }
        self.0.future.lock().as_mut().poll(cx).map(|res| {
            info!("thread {} exited with {:?}.", self.0.id, res);
            THREAD_POOL.lock().remove(&self.0.id);
            // add to zombie thread list, it will finally drop in idle thread
            ZOMBIES.lock().push(self.0.clone());
        })
    }
}

pub async fn idle() -> AcoreResult {
    loop {
        // drop all zombie threads and deallocate their root page tables
        ZOMBIES.lock().clear();
        if THREAD_POOL.lock().len() == 1 {
            info!("no threads to run, waiting for interrupt...");
            crate::arch::cpu::wait_for_interrupt();
        }
        yield_now().await?;
    }
}
