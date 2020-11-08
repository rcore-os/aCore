use spin::Mutex;

use crate::asynccall::AsyncCallBuffer;

#[derive(Default)]
pub struct OwnedResource {
    pub async_buf: Mutex<Option<AsyncCallBuffer>>,
}

#[derive(Default)]
pub struct SharedResource;
