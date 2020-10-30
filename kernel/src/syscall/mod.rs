mod consts;

use core::convert::TryFrom;

use crate::error::AcoreError;
use crate::task::Thread;
use consts::SyscallType as Sys;

pub struct Syscall<'a> {
    thread: &'a Thread,
}

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

        let ret = match sys_type {
            Sys::EXIT => {
                self.thread.exit();
                Ok(0)
            }
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
