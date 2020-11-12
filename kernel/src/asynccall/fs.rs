use super::{AsyncCall, AsyncCallResult};
use crate::fs::get_file_by_fd;
use crate::memory::uaccess::{UserInPtr, UserOutPtr};

impl AsyncCall {
    pub async fn async_read(
        &self,
        fd: usize,
        mut base: UserOutPtr<u8>,
        count: usize,
        _offset: usize,
    ) -> AsyncCallResult {
        let file = get_file_by_fd(fd);
        let mut buf = vec![0u8; count];
        let count = file.read(0, &mut buf)?;
        base.write_array(&buf[..count])?;
        Ok(count)
    }

    pub async fn async_write(
        &self,
        fd: usize,
        base: UserInPtr<u8>,
        count: usize,
        _offset: usize,
    ) -> AsyncCallResult {
        let file = get_file_by_fd(fd);
        let buf = base.read_array(count)?;
        file.write(0, &buf)
    }
}
