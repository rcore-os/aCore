mod file;
mod stdio;

use alloc::{sync::Arc, vec::Vec};
use core::fmt::{Debug, Formatter, Result};

use crate::error::{AcoreError, AcoreResult};
use crate::utils::IdAllocator;
use stdio::{Stdin, Stdout};

pub use file::{File, RAM_DISK};

pub trait GenericFile: Send + Sync + Debug {
    fn open(&self) -> AcoreResult {
        Ok(())
    }
    fn release(&self) -> AcoreResult {
        Ok(())
    }
    fn read(&self, _offset: usize, _buf: &mut [u8]) -> AcoreResult<usize> {
        Err(AcoreError::NotSupported)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> AcoreResult<usize> {
        Err(AcoreError::NotSupported)
    }
}

pub struct FileStruct {
    files: Vec<Option<Arc<dyn GenericFile>>>,
    fd_allocator: IdAllocator,
}

impl FileStruct {
    pub fn new(max_file_num: usize) -> AcoreResult<Self> {
        let mut files = Self {
            files: vec![None; max_file_num],
            fd_allocator: IdAllocator::new(0..max_file_num)?,
        };
        files.add_file(Arc::new(Stdin))?;
        files.add_file(Arc::new(Stdout))?;
        files.add_file(Arc::new(Stdout))?;
        Ok(files)
    }

    pub fn add_file(&mut self, file: Arc<dyn GenericFile>) -> AcoreResult<usize> {
        let fd = self.fd_allocator.alloc()?;
        debug_assert!(self.files[fd].is_none());
        self.files[fd] = Some(file);
        Ok(fd)
    }

    pub fn get_file(&self, fd: usize) -> AcoreResult<Arc<dyn GenericFile>> {
        if fd >= self.files.len() {
            return Err(AcoreError::BadFileDescriptor);
        }
        match &self.files[fd] {
            Some(f) => Ok(f.clone()),
            None => Err(AcoreError::BadFileDescriptor),
        }
    }

    pub fn remove_file(&mut self, fd: usize) -> AcoreResult {
        if fd >= self.files.len() || self.files[fd].is_none() {
            return Err(AcoreError::BadFileDescriptor);
        }
        self.files[fd] = None;
        self.fd_allocator.dealloc(fd);
        Ok(())
    }
}

impl Drop for FileStruct {
    fn drop(&mut self) {
        for file in &self.files {
            if let Some(f) = file {
                f.release().ok();
            }
        }
    }
}

impl Debug for FileStruct {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_map()
            .entries(
                self.files
                    .iter()
                    .enumerate()
                    .filter_map(|(i, f)| f.as_ref().map(|f| (i, f))),
            )
            .finish()
    }
}
