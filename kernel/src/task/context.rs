use alloc::boxed::Box;

use crate::error::{AcoreError, AcoreResult};
use crate::memory::{handle_page_fault, MMUFlags};

pub trait ThreadContext: core::fmt::Debug + Send + Sync {
    /// Create a new context and set entry pointer, stack point, etc.
    fn new(entry_pointer: usize, arg: usize, stack_point: usize, is_user: bool) -> Self;

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

pub fn handle_user_trap<C: ThreadContext>(trap: TrapReason, ctx: &mut Box<C>) -> AcoreResult {
    trace!("handle trap from user: {:#x?} {:#x?}", trap, ctx);
    let res = match trap {
        TrapReason::Syscall => handle_syscall(ctx),
        TrapReason::PageFault(addr, access_flags) => handle_page_fault(addr, access_flags),
        _ => {
            warn!("unhandled trap from user: {:#x?}", trap);
            Err(AcoreError::NotSupported)
        }
    };
    trace!("user trap end");
    res
}

fn handle_syscall<C: ThreadContext>(ctx: &mut Box<C>) -> AcoreResult {
    let num = ctx.get_syscall_num();
    let args = ctx.get_syscall_args();
    println!("SYSCALL {} {:?}", num, args);
    let ret = num + 1;
    ctx.set_syscall_ret(ret);
    super::current().set_need_sched();
    if num == 2344 {
        super::current().set_exited();
    }
    Ok(())
}
