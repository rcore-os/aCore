use alloc::sync::Arc;
use core::convert::TryFrom;

use crate::arch::syscall_ids::SyscallType as Sys;
use crate::asynccall::{AsyncCall, AsyncCallInfoUser};
use crate::error::{AcoreError, AcoreResult};
use crate::fs::get_file_by_fd;
use crate::memory::uaccess::{UserInPtr, UserOutPtr};
use crate::task::Thread;

pub struct Syscall<'a> {
    thread: &'a Arc<Thread>,
}

type SysResult = AcoreResult<usize>;

impl<'a> Syscall<'a> {
    pub fn new(thread: &'a Arc<Thread>) -> Self {
        Self { thread }
    }

    pub fn syscall(&self, num: u32, args: [usize; 6]) -> SysResult {
        if self.thread.is_exited() {
            return Err(AcoreError::BadState);
        }
        let sys_type = match Sys::try_from(num) {
            Ok(t) => t,
            Err(_) => {
                error!("invalid syscall number: {}", num);
                return Err(AcoreError::InvalidArgs);
            }
        };
        debug!("Syscall: {:?} => args={:x?}", sys_type, args);

        let [a0, a1, a2, a3, _a4, _a5] = args;
        let ret = match sys_type {
            Sys::READ => self.sys_read(a0, a1.into(), a2),
            Sys::WRITE => self.sys_write(a0, a1.into(), a2),
            Sys::SCHED_YIELD => self.sys_yield(),
            Sys::GETPID => self.sys_getpid(),
            Sys::EXIT => self.sys_exit(a0),
            Sys::SETUP_ASYNC_CALL => self.sys_setup_async_call(a0, a1, a2.into(), a3),
            _ => {
                warn!("syscall unimplemented: {:?}", sys_type);
                Err(AcoreError::NotSupported)
            }
        };

        if ret.is_err() {
            warn!("Syscall: {:?} <= {:?}", sys_type, ret);
        } else {
            info!("Syscall: {:?} <= {:?}", sys_type, ret);
        }
        ret
    }
}

impl Syscall<'_> {
    fn sys_read(&self, fd: usize, mut base: UserOutPtr<u8>, count: usize) -> SysResult {
        let file = get_file_by_fd(fd);
        let mut buf = vec![0u8; count];
        let count = file.read(0, &mut buf)?;
        base.write_array(&buf[..count])?;
        Ok(count)
    }

    fn sys_write(&self, fd: usize, base: UserInPtr<u8>, count: usize) -> SysResult {
        let file = get_file_by_fd(fd);
        let buf = base.read_array(count)?;
        file.write(0, &buf)
    }

    fn sys_yield(&self) -> SysResult {
        self.thread.set_need_sched();
        Ok(0)
    }

    fn sys_getpid(&self) -> SysResult {
        Ok(self.thread.id)
    }

    fn sys_exit(&self, code: usize) -> SysResult {
        self.thread.exit(code);
        Ok(0)
    }

    fn sys_setup_async_call(
        &self,
        req_capacity: usize,
        comp_capacity: usize,
        mut out_info: UserOutPtr<AsyncCallInfoUser>,
        info_size: usize,
    ) -> SysResult {
        if info_size != core::mem::size_of::<AsyncCallInfoUser>() {
            return Err(AcoreError::InvalidArgs);
        }
        let res = AsyncCall::setup(&self.thread, req_capacity, comp_capacity)?;
        info!(
            "setup_async_call: req_capacity={}, comp_capacity={}, out_info={:#x?}",
            req_capacity, comp_capacity, res
        );
        out_info.write(res)?;
        Ok(0)
    }
}
