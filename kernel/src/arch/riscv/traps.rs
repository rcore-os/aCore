use trapframe::TrapFrame;

use riscv::register::scause::{self, Exception as E, Interrupt as I, Trap};
use riscv::register::stval;

use crate::memory::MMUFlags;

/// handle interrupt from kernel
#[no_mangle]
extern "C" fn trap_handler(tf: &mut TrapFrame) {
    let scause = scause::read();
    let stval = stval::read();
    trace!(
        "handle trap from kernel @ CPU{}: {:?} ",
        super::cpu::id(),
        scause.cause()
    );
    match scause.cause() {
        Trap::Interrupt(I::SupervisorExternal) => {}
        Trap::Interrupt(I::SupervisorSoft) => ipi(),
        Trap::Interrupt(I::SupervisorTimer) => {}
        Trap::Exception(E::InstructionPageFault) => page_fault(stval, MMUFlags::EXECUTE, tf),
        Trap::Exception(E::LoadPageFault) => page_fault(stval, MMUFlags::READ, tf),
        Trap::Exception(E::StorePageFault) => page_fault(stval, MMUFlags::WRITE, tf),
        _ => error!("unhandled trap from kernel: {:?}", scause.cause()),
    }
    trace!("kernel trap end");
}

fn ipi() {
    debug!("IPI");
    super::sbi::clear_ipi();
}

fn page_fault(stval: usize, access_flags: MMUFlags, _tf: &mut TrapFrame) {
    let addr = stval;
    trace!("Page Fault @ {:#x} when {:?}", addr, access_flags);
    panic!("unhandled page fault");
}
