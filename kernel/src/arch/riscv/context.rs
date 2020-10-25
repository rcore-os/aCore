use trapframe::UserContext;

use riscv::register::scause::{self, Exception as E, Interrupt as I, Trap};

use crate::memory::MMUFlags;
use crate::task::context::{ThreadContext, TrapKind};

#[derive(Debug)]
pub struct ArchThreadContext {
    inner: UserContext,
}

impl ThreadContext for ArchThreadContext {
    fn new(entry_pointer: usize, arg: usize, stack_point: usize, is_user: bool) -> Self {
        let mut ctx = UserContext::default();
        ctx.set_ip(entry_pointer);
        ctx.set_sp(stack_point);
        ctx.general.a0 = arg;
        ctx.sstatus = 1 << 5; // SPIE
        if !is_user {
            ctx.sstatus |= 1 << 8; // SPP
        }
        Self { inner: ctx }
    }

    fn get_syscall_num(&self) -> usize {
        self.inner.get_syscall_num()
    }

    fn get_syscall_ret(&self) -> usize {
        self.inner.get_syscall_ret()
    }

    fn set_syscall_ret(&mut self, ret: usize) {
        self.inner.set_syscall_ret(ret)
    }

    fn get_syscall_args(&self) -> [usize; 6] {
        self.inner.get_syscall_args()
    }

    fn run(&mut self) -> TrapKind {
        self.inner.run();
        let scause = scause::read();
        match scause.cause() {
            Trap::Interrupt(I::SupervisorTimer) => TrapKind::Timer,
            Trap::Exception(E::UserEnvCall) => TrapKind::Syscall,
            Trap::Exception(E::InstructionPageFault) => {
                TrapKind::PageFault(MMUFlags::USER | MMUFlags::EXECUTE)
            }
            Trap::Exception(E::LoadPageFault) => {
                TrapKind::PageFault(MMUFlags::USER | MMUFlags::READ)
            }
            Trap::Exception(E::StorePageFault) => {
                TrapKind::PageFault(MMUFlags::USER | MMUFlags::WRITE)
            }
            _ => TrapKind::Unknown(scause.bits()),
        }
    }
}
