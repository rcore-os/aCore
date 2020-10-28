#![allow(dead_code)]

mod context;
mod thread;

use alloc::sync::Arc;

use crate::sched::executor;

pub use context::{ThreadContext, TrapReason};
pub use thread::Thread;

pub fn current<'a>() -> &'a Thread {
    let ptr = crate::arch::context::read_tls() as *const Thread;
    unsafe { &*ptr }
}

pub fn spawn(thread: Arc<Thread>) {
    info!(
        "spawn {} thread {}.",
        if thread.is_user { "user" } else { "kernel" },
        thread.id
    );
    executor::spawn(thread::ThreadSwitchFuture::new(thread));
}

pub fn init() {
    spawn(Thread::new_kernel(thread::idle()).unwrap());
    spawn(Thread::new_user(test_user_thread as usize, 2333).unwrap());
    spawn(Thread::new_user(test_user_thread as usize, 2336).unwrap());
    spawn(
        Thread::new_kernel(async move {
            for i in 0..20 {
                println!("TEST kernel thread {}", i);
                super::sched::yield_now().await?;
            }
            Ok(())
        })
        .unwrap(),
    );
}

pub fn run_forever() -> ! {
    executor::run_until_idle();
    unreachable!();
}

fn test_user_thread(arg: usize) -> ! {
    let a = [2, 3, 3, 4];
    let mut num = arg;
    loop {
        let mut ret = 0;
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
