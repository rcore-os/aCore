pub fn boot_cpu_id() -> usize {
    super::context::read_tls()
}

pub fn id() -> usize {
    let boot_id = boot_cpu_id();
    if boot_id < crate::consts::MAX_CPU_NUM {
        boot_id
    } else {
        unsafe { crate::task::current().cpu }
    }
}

pub fn wait_for_interrupt() {
    unsafe { riscv::asm::wfi() }
}

pub fn send_ipi(cpu_id: usize) {
    super::sbi::send_ipi(1 << cpu_id);
}
