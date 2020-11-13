use super::GenericFile;
use crate::error::{AcoreError, AcoreResult};

#[derive(Debug)]
pub struct Stdin;

#[derive(Debug)]
pub struct Stdout;

impl GenericFile for Stdin {
    fn read(&self, _offset: usize, _buf: &mut [u8]) -> AcoreResult<usize> {
        Err(AcoreError::NotSupported)
    }

    fn write(&self, _offset: usize, _buf: &[u8]) -> AcoreResult<usize> {
        Err(AcoreError::NotSupported)
    }
}

impl GenericFile for Stdout {
    fn read(&self, _offset: usize, _buf: &mut [u8]) -> AcoreResult<usize> {
        Err(AcoreError::NotSupported)
    }

    fn write(&self, _offset: usize, buf: &[u8]) -> AcoreResult<usize> {
        let s = unsafe { core::str::from_utf8_unchecked(buf) };
        print!("{}", s);
        Ok(buf.len())
    }
}
