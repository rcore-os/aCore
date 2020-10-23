use trapframe::TrapFrame;

use riscv::register::scause::{self, Exception as E, Interrupt as I, Trap};
use riscv::register::stval;

/// handle interrupt from kernel
#[no_mangle]
extern "C" fn trap_handler(_tf: &mut TrapFrame) {
    let scause = scause::read();
    let _stval = stval::read();
    trace!(
        "handle trap from kernel @ CPU{}: {:?} ",
        super::cpu::id(),
        scause.cause()
    );
    match scause.cause() {
        Trap::Interrupt(I::SupervisorExternal) => {}
        Trap::Interrupt(I::SupervisorSoft) => ipi(),
        Trap::Interrupt(I::SupervisorTimer) => {}
        Trap::Exception(E::InstructionPageFault)
        | Trap::Exception(E::LoadPageFault)
        | Trap::Exception(E::StorePageFault) => {}
        _ => error!("unhandled trap from kernel: {:?}", scause.cause()),
    }
    trace!("kernel trap end");
}

fn ipi() {
    debug!("IPI");
    super::sbi::clear_ipi();
}
