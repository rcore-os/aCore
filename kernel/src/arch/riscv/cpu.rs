pub fn id() -> usize {
    let boot_id = super::boot_cpu_id();
    if boot_id < crate::consts::MAX_CPU_NUM {
        boot_id
    } else {
        crate::task::current().cpu
    }
}

pub fn wait_for_interrupt() {
    unsafe { riscv::asm::wfi() }
}
