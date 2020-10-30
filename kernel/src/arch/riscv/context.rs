use riscv::register::{
    scause::{self, Exception as E, Interrupt as I, Trap},
    stval,
};
use trapframe::UserContext;

use crate::memory::MMUFlags;
use crate::task::{ThreadContext, TrapReason};

#[derive(Debug)]
pub struct ArchThreadContext {
    user: UserContext,
}

impl ThreadContext for ArchThreadContext {
    fn new(entry_pointer: usize, stack_point: usize) -> Self {
        let mut ctx = UserContext::default();
        ctx.set_ip(entry_pointer);
        ctx.set_sp(stack_point);
        ctx.sstatus = 1 << 5; // SPIE
        Self { user: ctx }
    }

    fn get_syscall_num(&self) -> usize {
        self.user.get_syscall_num()
    }

    fn get_syscall_ret(&self) -> usize {
        self.user.get_syscall_ret()
    }

    fn set_syscall_ret(&mut self, ret: usize) {
        self.user.set_syscall_ret(ret)
    }

    fn get_syscall_args(&self) -> [usize; 6] {
        self.user.get_syscall_args()
    }

    fn set_tls(&mut self, tls: usize) {
        self.user.set_tls(tls)
    }

    fn run(&mut self) -> TrapReason {
        self.user.run();
        let scause = scause::read();
        let stval = stval::read();
        match scause.cause() {
            Trap::Interrupt(I::SupervisorTimer) => TrapReason::Timer,
            Trap::Exception(E::UserEnvCall) => TrapReason::Syscall,
            Trap::Exception(E::InstructionPageFault) => {
                TrapReason::PageFault(stval, MMUFlags::USER | MMUFlags::EXECUTE)
            }
            Trap::Exception(E::LoadPageFault) => {
                TrapReason::PageFault(stval, MMUFlags::USER | MMUFlags::READ)
            }
            Trap::Exception(E::StorePageFault) => {
                TrapReason::PageFault(stval, MMUFlags::USER | MMUFlags::WRITE)
            }
            _ => TrapReason::Unknown(scause.bits()),
        }
    }

    fn end_trap(&mut self, trap: TrapReason) {
        if let TrapReason::Syscall = trap {
            self.user.sepc += 4;
        }
    }
}

pub fn read_tls() -> usize {
    let ptr: usize;
    unsafe { asm!("mv {0}, tp", out(reg) ptr) };
    ptr
}

pub unsafe fn write_tls(ptr: usize) {
    asm!("mv tp, {0}", in(reg) ptr);
}
