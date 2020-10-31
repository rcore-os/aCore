mod stdio;

use alloc::boxed::Box;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::error::AcoreResult;
use crate::memory::addr::{phys_to_virt, PhysAddr};
use crate::memory::{DEVICE_END, DEVICE_START};

pub struct Disk {
    data: &'static mut [u8],
    size: usize,
}

pub struct File {
    offset_in_disk: usize,
    size: usize,
}

pub trait GenericFile {
    fn read(&self, offset: usize, buf: &mut [u8]) -> AcoreResult<usize>;
    fn write(&self, offset: usize, buf: &[u8]) -> AcoreResult<usize>;
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
                size,
            }
        }
    }

    pub fn lookup(&mut self, _filename: &str) -> File {
        File::new(0, self.size)
    }
}

impl File {
    fn new(offset_in_disk: usize, size: usize) -> Self {
        Self {
            offset_in_disk,
            size,
        }
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

pub fn get_file_by_fd(fd: usize) -> Box<dyn GenericFile> {
    match fd {
        0 => Box::new(stdio::Stdin),
        1 => Box::new(stdio::Stdout),
        2 => Box::new(stdio::Stdout),
        _ => Box::new(RAM_DISK.lock().lookup("")),
    }
}
