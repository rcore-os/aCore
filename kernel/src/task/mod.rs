mod context;
mod resource;
mod thread;

use alloc::sync::Arc;

use crate::fs::RAM_DISK;
use crate::sched::executor;

pub use context::{ThreadContext, TrapReason};
pub use thread::Thread;

pub unsafe fn current<'a>() -> &'a Thread {
    let ptr = crate::arch::context::read_tls() as *const Thread;
    &*ptr
}

pub unsafe fn current_option<'a>() -> Option<&'a Thread> {
    let tls = crate::arch::context::read_tls();
    if tls < crate::config::CPU_NUM {
        None
    } else {
        let ptr = tls as *const Thread;
        Some(&*ptr)
    }
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
    let init_elf = RAM_DISK.lock().lookup("init");
    spawn(Thread::new_kernel(thread::idle()).unwrap());
    spawn(Thread::new_user(&init_elf, vec!["arg0".into(), "arg1".into()]).unwrap());
    spawn(Thread::new_user(&init_elf, vec!["arg2".into(), "arg3".into()]).unwrap());
    spawn(
        Thread::new_kernel(async move {
            for i in 0..20 {
                println!("TEST kernel thread {}", i);
                super::sched::yield_now().await;
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
