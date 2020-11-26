use super::{AsyncCall, AsyncCallResult};
use crate::error::AcoreError;
use crate::memory::uaccess::{UserInPtr, UserOutPtr};

impl AsyncCall {
    pub async fn async_read(
        &self,
        fd: usize,
        mut base: UserOutPtr<u8>,
        count: usize,
        offset: usize,
    ) -> AsyncCallResult {
        let file = self.thread.shared_res.files.lock().get_file(fd)?;
        let mut buf = vec![0u8; count];
        let count = file.read(offset, &mut buf)?;
        base.write_array(&buf[..count])?;
        Ok(count)
    }

    pub async fn async_write(
        &self,
        fd: usize,
        base: UserInPtr<u8>,
        count: usize,
        offset: usize,
    ) -> AsyncCallResult {
        let file = self.thread.shared_res.files.lock().get_file(fd)?;
        let buf = base.read_array(count)?;
        file.write(offset, &buf)
    }

    pub async fn async_open(&self, path: UserInPtr<u8>, flags: usize) -> AsyncCallResult {
        println!("async_open {:x?} {:x?}", path, flags);
        Err(AcoreError::NotSupported)
    }

    pub async fn async_close(&self, fd: usize) -> AsyncCallResult {
        let file = self.thread.shared_res.files.lock().get_file(fd)?;
        file.release()?;
        self.thread.shared_res.files.lock().remove_file(fd)?;
        Ok(0)
    }
}
