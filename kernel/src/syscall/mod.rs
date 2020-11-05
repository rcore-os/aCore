use core::convert::TryFrom;

use crate::arch::syscall_ids::SyscallType as Sys;
use crate::asynccall::{self, AsyncCallInfo};
use crate::error::{AcoreError, AcoreResult};
use crate::fs::get_file_by_fd;
use crate::memory::uaccess::{UserInPtr, UserOutPtr};
use crate::task::Thread;

pub struct Syscall<'a> {
    thread: &'a Thread,
}

type SysResult = AcoreResult<usize>;

impl<'a> Syscall<'a> {
    pub fn new(thread: &'a Thread) -> Self {
        Self { thread }
    }

    pub fn syscall(&mut self, num: u32, args: [usize; 6]) -> isize {
        let sys_type = match Sys::try_from(num) {
            Ok(t) => t,
            Err(_) => {
                error!("invalid syscall number: {}", num);
                return -(AcoreError::InvalidArgs as isize);
            }
        };
        debug!("{:?} => args={:x?}", sys_type, args);

        let [a0, a1, a2, a3, _a4, _a5] = args;
        let ret = match sys_type {
            Sys::READ => self.sys_read(a0, a1.into(), a2),
            Sys::WRITE => self.sys_write(a0, a1.into(), a2),
            Sys::SCHED_YIELD => self.sys_yield(),
            Sys::GETPID => self.sys_getpid(),
            Sys::EXIT => self.sys_exit(a0),
            Sys::SETUP_ASYNC_CALL => self.sys_setup_async_call(a0, a1, a2 as _, a3.into()),
            _ => {
                warn!("syscall unimplemented: {:?}", sys_type);
                Err(AcoreError::NotSupported)
            }
        };

        info!("{:?} <= {:?}", sys_type, ret);
        match ret {
            Ok(code) => code as isize,
            Err(err) => -(err as isize),
        }
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
        arg0: usize,
        arg1: usize,
        flags: u64,
        mut out_info: UserOutPtr<AsyncCallInfo>,
    ) -> SysResult {
        let res = asynccall::setup_async_call(&self.thread, arg0, arg1, flags)?;
        info!(
            "setup_async_call: arg0={}, arg1={}, flags={:#x?}, out_info={:#x?}",
            arg0, arg1, flags, res
        );
        out_info.write(res)?;
        Ok(0)
    }
}
