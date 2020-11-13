use spin::Mutex;

use crate::asynccall::AsyncCallBuffer;
use crate::error::AcoreResult;
use crate::fs::FileStruct;

#[derive(Default, Debug)]
pub struct OwnedResource {
    pub async_buf: Mutex<Option<AsyncCallBuffer>>,
}

#[derive(Debug)]
pub struct SharedResource {
    pub files: Mutex<FileStruct>,
}

impl SharedResource {
    pub fn new() -> AcoreResult<Self> {
        Ok(Self {
            files: Mutex::new(FileStruct::new(res_limit::MAX_FILE_NUM)?),
        })
    }
}

pub mod res_limit {
    pub const MAX_FILE_NUM: usize = 256;
    pub const MAX_ASYNC_CALL_ENTRY_NUM: usize = 32768;
}
