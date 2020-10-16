#![no_std]
#![no_main]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(global_asm)]

mod consts;
mod lang;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
mod arch;

use core::sync::atomic::{spin_loop_hint, AtomicBool, Ordering};

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn start_kernel(arg0: usize, arg1: usize) -> ! {
    if arch::cpu::id() == consts::BOOTSTRAP_CPU_ID {
        arch::primary_init(arg0, arg1);
        AP_CAN_INIT.store(true, Ordering::Relaxed);
    } else {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {
            spin_loop_hint();
        }
        arch::secondary_init(arg0, arg1);
    }
    match arch::cpu::id() {
        consts::NORMAL_CPU_ID => normal_main(),
        consts::IO_CPU_ID => io_main(),
        _ => loop {},
    }
}

pub fn normal_main() -> ! {
    arch::io::print("Hello, normal CPU!\n");
    loop {}
}

pub fn io_main() -> ! {
    arch::io::print("Hello, I/O CPU!\n");
    loop {}
}
