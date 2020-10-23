use alloc::boxed::Box;
use alloc::sync::Arc;

use super::thread::Thread;
use crate::error::AcoreResult;

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

    /// Go to user space with the context, and come back when a trap occurs.
    /// Returns the trap kind.
    ///
    /// On return, the context will be reset to the status before the trap.
    /// Trap reason and error code will be returned.
    fn run(&mut self) -> TrapKind;
}

#[derive(Debug)]
pub enum TrapKind {
    Syscall,
    Timer,
    PageFault,
    Irq,
    Unknown(usize),
}

pub fn handle_user_trap<C: ThreadContext>(
    _thread: &Arc<Thread<C>>,
    trap: TrapKind,
    ctx: &mut Box<C>,
) -> AcoreResult {
    trace!("handle trap from user: {:#x?} {:#x?}", trap, ctx);
    match trap {
        TrapKind::Syscall => info!(
            "SYSCALL {} {:#x?}",
            ctx.get_syscall_num(),
            ctx.get_syscall_args()
        ),
        _ => error!("unhandled trap from user: {:#x?}", trap),
    }
    trace!("user trap end");
    Ok(())
}
