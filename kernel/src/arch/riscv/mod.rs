pub mod cpu;
pub mod io;

global_asm!(include_str!("boot/entry.S"));

pub fn primary_init(_arg0: usize, _arg1: usize) {
    io::print("Hello, CPU 0!\n");
}

pub fn secondary_init(_arg0: usize, _arg1: usize) {
    io::print("Hello, CPU 1!\n");
}
