use alloc::boxed::Box;

use super::Thread;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::{MMUFlags, VirtAddr};
use crate::syscall::Syscall;

pub trait ThreadContext: core::fmt::Debug + Send + Sync {
    /// Create a new context and set entry pointer, stack point, etc.
    fn new(entry_pointer: usize, stack_point: usize) -> Self;

    /// Get number of syscall.
    fn get_syscall_num(&self) -> usize;

    /// Get return value of syscall.
    fn get_syscall_ret(&self) -> usize;

    /// Set return value of syscall.
    fn set_syscall_ret(&mut self, ret: usize);

    /// Get syscall args
    fn get_syscall_args(&self) -> [usize; 6];

    /// Set thread local storage pointer
    fn set_tls(&mut self, tls: usize);

    /// Go to user space with the context, and come back when a trap occurs.
    /// Returns the trap kind.
    ///
    /// On return, the context will be reset to the status before the trap.
    /// Trap reason and error code will be returned.
    fn run(&mut self) -> TrapReason;

    /// Do something at the end of the trap, such as increasing PC.
    fn end_trap(&mut self, trap: TrapReason);
}

#[derive(Debug, Clone, Copy)]
pub enum TrapReason {
    Syscall,
    Timer,
    PageFault(usize, MMUFlags),
    Irq(usize),
    Unknown(usize),
}

impl Thread {
    pub fn handle_user_trap(
        &self,
        trap: TrapReason,
        ctx: &mut Box<impl ThreadContext>,
    ) -> AcoreResult {
        trace!("handle trap from user: {:#x?} {:#x?}", trap, ctx);
        let res = match trap {
            TrapReason::Syscall => self.handle_syscall(ctx),
            TrapReason::PageFault(addr, access_flags) => self.handle_page_fault(addr, access_flags),
            _ => {
                warn!("unhandled trap from user: {:#x?}", trap);
                Err(AcoreError::NotSupported)
            }
        };
        trace!("user trap end");
        res
    }

    fn handle_page_fault(&self, vaddr: VirtAddr, access_flags: MMUFlags) -> AcoreResult {
        debug!("page fault @ {:#x} with access {:?}", vaddr, access_flags);
        self.vm.lock().handle_page_fault(vaddr, access_flags)
    }

    fn handle_syscall(&self, ctx: &mut Box<impl ThreadContext>) -> AcoreResult {
        let num = ctx.get_syscall_num() as u32;
        let args = ctx.get_syscall_args();
        let ret = Syscall::new(&self).syscall(num, args) as usize;
        ctx.set_syscall_ret(ret);
        Ok(())
    }
}
