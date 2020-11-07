use super::{AsyncCall, AsyncCallResult};
use crate::memory::uaccess::{UserInPtr, UserOutPtr};

impl AsyncCall {
    pub async fn async_read(
        &self,
        fd: usize,
        base: UserOutPtr<u8>,
        count: usize,
        offset: usize,
    ) -> AsyncCallResult {
        println!(
            "READ {} {:?} {:x?} {:?} {:?}",
            self.thread.id, fd, base, count, offset
        );
        Ok(0)
    }

    pub async fn async_write(
        &self,
        fd: usize,
        base: UserInPtr<u8>,
        count: usize,
        offset: usize,
    ) -> AsyncCallResult {
        println!(
            "WRITE {} {:?} {:x?} {:?} {:?}",
            self.thread.id, fd, base, count, offset
        );
        Ok(0)
    }
}
