use alloc::string::String;
use core::fmt::{Debug, Formatter, Result};

use spin::Mutex;

use super::GenericFile;
use crate::error::AcoreResult;
use crate::memory::addr::{phys_to_virt, PhysAddr};
use crate::memory::{DEVICE_END, DEVICE_START};

pub const ELF_SIZE: usize = (DEVICE_END - DEVICE_START) >> 1;
pub const MEMORY_FILE_START: usize = DEVICE_START + ELF_SIZE;
pub const MEMORY_FILE_END: usize = DEVICE_END;
pub const MEMORY_FILE_MAX_COUNT: usize = (MEMORY_FILE_END - MEMORY_FILE_START) / MEMORY_FILE_SIZE;
pub const MEMORY_FILE_SIZE: usize = 0x100_0000;

pub struct Disk {
    data: &'static mut [u8],
    _size: usize,
}

pub struct File {
    path: String,
    offset_in_disk: usize,
    size: usize,
}

lazy_static! {
    pub static ref RAM_DISK: Mutex<Disk> =
        Mutex::new(Disk::new(DEVICE_START, DEVICE_END - DEVICE_START));
}

impl Disk {
    fn new(start_paddr: PhysAddr, size: usize) -> Self {
        unsafe {
            Self {
                data: core::slice::from_raw_parts_mut(phys_to_virt(start_paddr) as *mut u8, size),
                _size: size,
            }
        }
    }

    pub fn lookup(&mut self, path: &str) -> File {
        File::new(path.into(), 0, ELF_SIZE)
    }
}

impl File {
    fn new(path: String, offset_in_disk: usize, size: usize) -> Self {
        Self {
            path,
            offset_in_disk,
            size,
        }
    }

    pub fn new_memory_file(path: String) -> AcoreResult<Self> {
        let id = path.len() as usize % MEMORY_FILE_MAX_COUNT;
        Ok(File::new(path, id * MEMORY_FILE_SIZE, MEMORY_FILE_SIZE))
    }

    pub fn as_slice_mut(&self) -> &'static mut [u8] {
        let ptr = RAM_DISK.lock().data.as_mut_ptr();
        unsafe { core::slice::from_raw_parts_mut(ptr.add(self.offset_in_disk), self.size) }
    }
}

impl GenericFile for File {
    fn read(&self, offset: usize, buf: &mut [u8]) -> AcoreResult<usize> {
        let len = buf.len();
        let offset = offset + self.offset_in_disk;
        buf.copy_from_slice(&RAM_DISK.lock().data[offset..offset + len]);
        Ok(len)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> AcoreResult<usize> {
        let len = buf.len();
        let offset = offset + self.offset_in_disk;
        RAM_DISK.lock().data[offset..offset + len].copy_from_slice(buf);
        Ok(len)
    }
}

impl Debug for File {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("File").field("path", &self.path).finish()
    }
}
