#![allow(dead_code)]

pub mod context;
pub mod thread;

use alloc::sync::Arc;

use crate::error::AcoreResult;
use thread::Thread;

fn test_new_thread() -> AcoreResult {
    let t = Thread::new_kernel(test_thread, 2333)?;
    info!("{:x?}", t);
    spawn(t);
    Ok(())
}

pub fn init() {
    test_new_thread().unwrap();
}

pub fn spawn(thread: Arc<Thread>) {
    thread.run().unwrap();
}

fn test_thread(arg: usize) -> ! {
    println!("Hello kernel thread! {}", arg);
    unsafe { asm!("ecall", in("a7") 93, in("a0") 2,in("a1") 3) }
    loop {}
}
