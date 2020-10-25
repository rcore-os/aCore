use riscv::register::{
    scause::{self, Exception as E, Interrupt as I, Trap},
    stval,
};
use trapframe::UserContext;

use crate::memory::MMUFlags;
use crate::task::context::{ThreadContext, TrapKind};

#[derive(Debug)]
pub struct ArchThreadContext {
    user: UserContext,
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

    fn run(&mut self) -> TrapKind {
        self.user.run();
        let scause = scause::read();
        let stval = stval::read();
        match scause.cause() {
            Trap::Interrupt(I::SupervisorTimer) => TrapKind::Timer,
            Trap::Exception(E::UserEnvCall) => TrapKind::Syscall,
            Trap::Exception(E::InstructionPageFault) => {
                TrapKind::PageFault(stval, MMUFlags::USER | MMUFlags::EXECUTE)
            }
            Trap::Exception(E::LoadPageFault) => {
                TrapKind::PageFault(stval, MMUFlags::USER | MMUFlags::READ)
            }
            Trap::Exception(E::StorePageFault) => {
                TrapKind::PageFault(stval, MMUFlags::USER | MMUFlags::WRITE)
            }
            _ => TrapKind::Unknown(scause.bits()),
        }
    }

    fn end_trap(&mut self, trap: TrapKind) {
        match trap {
            TrapKind::Syscall => self.user.sepc += 4,
            _ => {}
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
