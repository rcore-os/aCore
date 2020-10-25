#![allow(dead_code)]

pub mod context;
pub mod thread;

use alloc::sync::Arc;

use crate::error::AcoreResult;
use thread::Thread;

fn test_new_thread() -> AcoreResult {
    let t = Thread::new_user(test_user_thread, 2333)?;
    spawn(t);
    Ok(())
}

pub fn init() {
    test_new_thread().unwrap();
}

pub fn spawn(thread: Arc<Thread>) {
    thread.run().unwrap();
}

pub fn current<'a>() -> &'a Thread {
    let ptr = crate::arch::context::read_tls() as *const Thread;
    unsafe { &*ptr }
}

fn test_user_thread(arg: usize) -> ! {
    let mut num = 2333;
    let a = [2, 3, 3, 4];
    loop {
        let mut ret = arg;
        unsafe {
            asm!("ecall",
                in("a7") num,
                inlateout("a0") ret,
                in("a1") &a[0],
                in("a2") &a[1],
                in("a3") &a[2],
                in("a4") &a[3],
                out("a5") _,
            )
        };
        num = ret;
    }
}
