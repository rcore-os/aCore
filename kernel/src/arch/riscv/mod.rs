pub mod cpu;
pub mod io;
mod sbi;

global_asm!(include_str!("boot/entry.S"));

pub fn primary_init(arg0: usize, _arg1: usize) {
    println!("Hello, CPU {}!", arg0);
}

pub fn secondary_init(arg0: usize, _arg1: usize) {
    println!("Hello, CPU {}!", arg0);
}
