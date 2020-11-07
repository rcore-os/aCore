use spin::Once;

use crate::asynccall::AsyncCallBuffer;

pub struct OwnedResource {
    pub async_buf: Once<AsyncCallBuffer>,
}

#[derive(Default)]
pub struct SharedResource;

impl OwnedResource {
    pub fn new() -> Self {
        Self {
            async_buf: Once::new(),
        }
    }
}
