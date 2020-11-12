pub fn id() -> usize {
    crate::task::PerCpu::id()
}

pub fn read_tls() -> usize {
    let tls: usize;
    unsafe { asm!("mv {0}, tp", out(reg) tls) };
    tls
}

pub unsafe fn write_tls(tls: usize) {
    asm!("mv tp, {0}", in(reg) tls);
}

pub fn wait_for_interrupt() {
    unsafe { riscv::asm::wfi() }
}

pub fn send_ipi(cpu_id: usize) {
    super::sbi::send_ipi(1 << cpu_id);
}
