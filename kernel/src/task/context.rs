use alloc::sync::Arc;

use super::thread::Thread;
use crate::error::AcoreResult;
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
    fn run(&mut self) -> TrapKind;

    /// Do something at the end of the trap, such as increasing PC.
    fn end_trap(&mut self, trap: TrapKind);
}

#[derive(Debug, Clone, Copy)]
pub enum TrapKind {
    Syscall,
    Timer,
    PageFault(usize, MMUFlags),
    Irq(usize),
    Unknown(usize),
}

pub fn handle_user_trap<C: ThreadContext>(
    _thread: &Arc<Thread<C>>,
    trap: TrapKind,
    ctx: &mut C,
) -> AcoreResult {
    trace!("handle trap from user: {:#x?} {:#x?}", trap, ctx);
    match trap {
        TrapKind::Syscall => handle_syscall(ctx),
        TrapKind::PageFault(addr, access_flags) => handle_page_fault(addr, access_flags),
        _ => error!("unhandled trap from user: {:#x?}", trap),
    }
    trace!("user trap end");
    Ok(())
}

fn handle_syscall<C: ThreadContext>(ctx: &mut C) {
    let num = ctx.get_syscall_num();
    let args = ctx.get_syscall_args();
    info!("SYSCALL {} {:?}", num, args);
    let ret = num + 1;
    ctx.set_syscall_ret(ret);
}
