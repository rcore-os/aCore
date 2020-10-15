#![no_std]
#![no_main]
#![feature(llvm_asm)]
#![feature(global_asm)]

mod lang;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
mod arch;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    arch::main();
    loop {}
}
