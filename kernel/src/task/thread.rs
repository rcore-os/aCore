use alloc::collections::BTreeMap;
use alloc::{boxed::Box, sync::Arc};

use lazy_static::lazy_static;
use spin::Mutex;

use super::context::{handle_user_trap, ThreadContext};
use crate::arch::context::ArchThreadContext;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::addr::virt_to_phys;
use crate::memory::areas::{PmAreaDelay, VmArea};
use crate::memory::{MMUFlags, MemorySet, PAGE_SIZE, USER_STACK_OFFSET, USER_STACK_SIZE};
use crate::utils::IdAllocator;

#[derive(Debug, Default)]
struct ThreadState {
    need_sched: bool,
    exited: bool,
}

#[derive(Debug)]
pub struct Thread<C: ThreadContext = ArchThreadContext> {
    pub id: usize,
    pub cpu: usize,
    pub vm: Arc<Mutex<MemorySet>>,
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
            cpu: crate::arch::cpu::id(),
            vm: Arc::new(Mutex::new(MemorySet::new_user())),
            context: Mutex::new(None),
            state: Mutex::new(ThreadState::default()),
        });
        THREAD_POOL.lock().insert(t.id, t.clone());
        Ok(t)
    }

    pub fn exit_by_id(tid: usize) {
        THREAD_POOL.lock().remove(&tid);
    }

    pub fn exit(&self) {
        Self::exit_by_id(self.id)
    }

    pub fn tls_ptr(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self) as usize
    }

    pub fn new_user(entry: fn(usize) -> !, arg: usize) -> AcoreResult<Arc<Self>> {
        let th = Self::new()?;

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
        th.vm.lock().push(stack)?;

        // test text segment
        let text_start_paddr = virt_to_phys(entry as usize);
        let text_end_paddr = text_start_paddr + PAGE_SIZE;
        let text = VmArea::from_fixed_pma(
            text_start_paddr,
            text_end_paddr,
            0,
            MMUFlags::READ | MMUFlags::EXECUTE | MMUFlags::USER,
            "text",
        )?;
        th.vm.lock().push(text)?;

        let ctx = ArchThreadContext::new(virt_to_phys(entry as usize), arg, stack_top, true);
        *th.context.lock() = Some(Box::from(ctx));
        debug!("new user thread: {:#x?}", th);
        Ok(th)
    }

    pub async fn run(self: &Arc<Self>) -> AcoreResult {
        // FIXME
        for _ in 0..10 {
            let mut ctx = self.context.lock().take().ok_or(AcoreError::BadState)?;
            let trap = ctx.run();
            handle_user_trap(self, trap, &mut ctx)?;
            ctx.end_trap(trap);
            *self.context.lock() = Some(ctx);

            let state = self.state.lock();
            if state.exited {
                break;
            }
        }
        Ok(())
    }
}

impl<C: ThreadContext> Drop for Thread<C> {
    fn drop(&mut self) {
        debug!("drop thread: {:#x?}", self);
        TID_ALLOCATOR.lock().dealloc(self.id);
    }
}
