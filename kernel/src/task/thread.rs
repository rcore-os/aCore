use alloc::collections::BTreeMap;
use alloc::{boxed::Box, sync::Arc};
use core::fmt::{Debug, Formatter, Result};

use lazy_static::lazy_static;
use spin::Mutex;

use super::context::{handle_user_trap, ThreadContext};
use crate::arch::context::ArchThreadContext;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::areas::{PmAreaDelay, VmArea};
use crate::memory::{MMUFlags, MemorySet, PAGE_SIZE, USER_STACK_OFFSET, USER_STACK_SIZE};
use crate::utils::IdAllocator;

#[derive(Debug)]
struct ThreadState {}

pub struct Thread<C: ThreadContext = ArchThreadContext> {
    pub id: usize,
    vm: Arc<Mutex<MemorySet>>,
    context: Mutex<Option<Box<C>>>,
    state: Mutex<ThreadState>,
}

lazy_static! {
    #[repr(align(64))]
    static ref TID_ALLOCATOR: Mutex<IdAllocator> = Mutex::new(IdAllocator::new(1..65536));
    #[repr(align(64))]
    static ref THREAD_POOL: Mutex<BTreeMap<usize, Arc<Thread>>> = Mutex::new(BTreeMap::new());
}

impl Thread {
    fn new() -> AcoreResult<Arc<Self>> {
        let t = Arc::new(Self {
            id: TID_ALLOCATOR.lock().alloc()?,
            vm: Arc::new(Mutex::new(MemorySet::new())),
            context: Mutex::new(None),
            state: Mutex::new(ThreadState {}),
        });
        THREAD_POOL.lock().insert(t.id, t.clone());
        Ok(t)
    }

    pub fn exit(tid: usize) {
        THREAD_POOL.lock().remove(&tid);
    }

    pub fn new_kernel(entry: fn(usize) -> !, arg: usize) -> AcoreResult<Arc<Self>> {
        extern "C" {
            fn boot_stack_top();
        }
        let th = Self::new()?;

        let stack_bottom = USER_STACK_OFFSET;
        let stack_top = stack_bottom + USER_STACK_SIZE;
        let mut pma = PmAreaDelay::new(USER_STACK_SIZE)?;
        pma.pre_alloc(USER_STACK_SIZE - PAGE_SIZE, PAGE_SIZE)?;
        let stack = VmArea::new(
            stack_bottom,
            stack_top,
            MMUFlags::READ | MMUFlags::WRITE,
            Arc::new(Mutex::new(pma)),
            "stack",
        )?;
        th.vm.lock().push(stack)?;

        let ctx = ArchThreadContext::new(entry as usize, arg, stack_top, false); // TODO: kernel statck
        *th.context.lock() = Some(Box::from(ctx));
        Ok(th)
    }

    pub fn run(self: &Arc<Self>) -> AcoreResult {
        loop {
            let mut ctx = self.context.lock().take().ok_or(AcoreError::BadState)?;
            let trap = ctx.run();
            handle_user_trap(self, trap, &mut ctx)?;
            *self.context.lock() = Some(ctx);
        }
    }
}

impl<C: ThreadContext> Debug for Thread<C> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Thread")
            .field("id", &self.id)
            .field("vm", &self.vm.lock())
            .field("context", &self.context.lock())
            .field("state", &self.state.lock())
            .finish()
    }
}

impl<C: ThreadContext> Drop for Thread<C> {
    fn drop(&mut self) {
        debug!("Thread {} dropped", self.id);
        TID_ALLOCATOR.lock().dealloc(self.id);
    }
}
