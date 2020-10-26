use alloc::{boxed::Box, sync::Arc};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use spin::Mutex;

use super::Thread;
use crate::arch::context::write_tls;
use crate::error::AcoreResult;

pub(super) struct ThreadRunnerFuture {
    inner: Mutex<Pin<Box<dyn Future<Output = AcoreResult> + Send>>>,
    thread: Arc<Thread>,
}

impl ThreadRunnerFuture {
    pub fn new(thread: Arc<Thread>) -> Self {
        let tmp = thread.clone();
        let future = async move { tmp.run().await };
        Self {
            inner: Mutex::new(Box::pin(future)),
            thread,
        }
    }
}

impl Future for ThreadRunnerFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            write_tls(self.thread.tls_ptr());
            self.thread.vm.lock().activate();
        }
        let res = self.inner.lock().as_mut().poll(cx).map(|res| {
            info!("thread {} exited with {:?}", self.thread.id, res);
            self.thread.exit();
        });
        unsafe { write_tls(self.thread.cpu) };
        res
    }
}
