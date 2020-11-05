use alloc::boxed::Box;

use crate::asynccall::AsyncCallBuffer;
use crate::error::{AcoreError, AcoreResult};
use crate::memory::addr::PageAligned;

pub struct OwnedResource {
    pub async_buf: Option<Box<PageAligned<AsyncCallBuffer>>>,
}

#[derive(Default)]
pub struct SharedResource;

impl Default for OwnedResource {
    fn default() -> Self {
        Self { async_buf: None }
    }
}

impl OwnedResource {
    pub fn alloc_async_call_buffer(
        &mut self,
    ) -> AcoreResult<Option<&Box<PageAligned<AsyncCallBuffer>>>> {
        if !self.async_buf.is_none() {
            return Err(AcoreError::AlreadyExists);
        }
        self.async_buf = Some(Box::new(PageAligned::new(AsyncCallBuffer::new())));
        Ok(self.async_buf.as_ref())
    }
}
