mod context;
mod resource;
mod thread;

use alloc::sync::Arc;
use core::future::Future;

use crate::arch::cpu;
use crate::config::CPU_NUM;
use crate::fs::RAM_DISK;
use crate::sched::Executor;

pub use context::{ThreadContext, TrapReason};
pub use thread::Thread;

pub const MAX_CPU_NUM: usize = 256;

#[repr(align(256))]
#[derive(Default)]
pub struct PerCpu {
    thread: Option<Arc<Thread>>,
    executor: Executor,
}

lazy_static! {
    static ref PER_CPU: [PerCpu; CPU_NUM] = Default::default();
}

impl PerCpu {
    /// Get the CPU id from lower bits of the TLS register.
    pub fn id() -> usize {
        cpu::read_tls() & (MAX_CPU_NUM - 1)
    }

    /// Get the `PerCpu` struct from `cpu_id`.
    pub fn from_cpu_id<'a>(cpu_id: usize) -> &'a Self {
        &PER_CPU[cpu_id]
    }

    /// Get the `PerCpu` struct from the current CPU id.
    pub fn from_current_cpu_id<'a>() -> &'a Self {
        &PER_CPU[Self::id()]
    }

    /// Get the `PerCpu` struct from high bits of the TLS register. It performs less cycles than
    /// the `from_current_cpu_id()` function.
    ///
    /// # Safty
    ///
    /// This function unsafe because the TLS register may contains an invalid address of `PerCpu`.
    pub unsafe fn from_tls<'a>() -> &'a Self {
        let ptr = cpu::read_tls() & !(MAX_CPU_NUM - 1);
        debug_assert!(ptr > 0);
        &*(ptr as *const Self)
    }

    /// Get the mutable `PerCpu` struct from high bits of the TLS register. It performs less cycles
    /// than the `from_current_cpu_id()` function.
    ///
    /// # Safty
    ///
    /// This function unsafe because the TLS register may contains an invalid address of `PerCpu`.
    pub unsafe fn from_tls_mut<'a>() -> &'a mut Self {
        let ptr = cpu::read_tls() & !(MAX_CPU_NUM - 1);
        debug_assert!(ptr > 0);
        &mut *(ptr as *mut Self)
    }

    /// Change the `thread` field of the current `PerCpu` struct.
    pub fn set_current_thread(thread: &Arc<Thread>) {
        unsafe { Self::from_tls_mut().thread = Some(thread.clone()) }
    }

    pub fn thread_unwrap(&self) -> &Arc<Thread> {
        self.thread.as_ref().expect("no threads run in current CPU")
    }

    pub fn thread(&self) -> Option<&Arc<Thread>> {
        self.thread.as_ref()
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        self.executor.spawn(future)
    }

    pub fn run_until_idle(&self) {
        self.executor.run_until_idle()
    }
}

fn spawn(thread: Arc<Thread>) {
    info!(
        "spawn {} thread {}.",
        if thread.is_user { "user" } else { "kernel" },
        thread.id
    );
    PerCpu::from_current_cpu_id().spawn(thread::ThreadSwitchFuture::new(thread));
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
    PerCpu::from_current_cpu_id().run_until_idle();
    unreachable!();
}
