use core::fmt::{Arguments, Result, Write};

use spin::Mutex;

fn console_putchar(ch: u8) {
    let _ret: usize;
    let arg0: usize = ch as usize;
    let arg1: usize = 0;
    let arg2: usize = 0;
    let which: usize = 1;
    unsafe {
        llvm_asm!("ecall"
             : "={x10}" (_ret)
             : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (which)
             : "memory"
             : "volatile"
        );
    }
}

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            if c == 127 {
                console_putchar(8);
                console_putchar(b' ');
                console_putchar(8);
            } else {
                console_putchar(c);
            }
        }
        Ok(())
    }
}

pub fn putfmt(fmt: Arguments) {
    static CONSOLE: Mutex<Console> = Mutex::new(Console);
    CONSOLE.lock().write_fmt(fmt).unwrap();
}
