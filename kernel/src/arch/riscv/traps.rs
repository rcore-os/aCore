use trapframe::TrapFrame;

use riscv::register::scause::{self, Exception as E, Interrupt as I, Trap};
use riscv::register::stval;

use crate::memory::{handle_page_fault, MMUFlags};

/// handle interrupt from kernel
#[no_mangle]
extern "C" fn trap_handler(_tf: &mut TrapFrame) {
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
        Trap::Exception(E::InstructionPageFault) => {
            handle_page_fault(stval, MMUFlags::EXECUTE).unwrap()
        }
        Trap::Exception(E::LoadPageFault) => handle_page_fault(stval, MMUFlags::READ).unwrap(),
        Trap::Exception(E::StorePageFault) => handle_page_fault(stval, MMUFlags::WRITE).unwrap(),
        _ => panic!("unhandled trap from kernel: {:?}", scause.cause()),
    }
    trace!("kernel trap end");
}

fn ipi() {
    debug!("IPI");
    super::sbi::clear_ipi();
}
