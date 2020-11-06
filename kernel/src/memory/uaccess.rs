use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result};
use core::marker::PhantomData;

use super::{VirtAddr, USER_VIRT_ADDR_LIMIT};
use crate::arch::memory::with_user_access;
use crate::error::{AcoreError, AcoreResult};

fn user_access_ok(uvaddr_start: VirtAddr, size: usize) -> bool {
    size <= USER_VIRT_ADDR_LIMIT && uvaddr_start <= USER_VIRT_ADDR_LIMIT - size
}

unsafe fn copy_from_user<T>(kdst: *mut T, usrc: *const T, len: usize) -> AcoreResult {
    // TODO: handle kernel page fault
    with_user_access(|| kdst.copy_from_nonoverlapping(usrc, len));
    Ok(())
}

unsafe fn copy_to_user<T>(udst: *mut T, ksrc: *const T, len: usize) -> AcoreResult {
    // TODO: handle kernel page fault
    with_user_access(|| udst.copy_from_nonoverlapping(ksrc, len));
    Ok(())
}

#[repr(C)]
pub struct UserPtr<T, P: Policy> {
    ptr: *mut T,
    mark: PhantomData<P>,
}

pub trait Policy {}
pub trait Read: Policy {}
pub trait Write: Policy {}
pub enum In {}
pub enum Out {}
pub enum InOut {}

impl Policy for In {}
impl Policy for Out {}
impl Policy for InOut {}
impl Read for In {}
impl Write for Out {}
impl Read for InOut {}
impl Write for InOut {}

pub type UserInPtr<T> = UserPtr<T, In>;
pub type UserOutPtr<T> = UserPtr<T, Out>;
pub type UserInOutPtr<T> = UserPtr<T, InOut>;

impl<T, P: Policy> Debug for UserPtr<T, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:?}", self.ptr)
    }
}

impl<T, P: Policy> From<VirtAddr> for UserPtr<T, P> {
    fn from(x: VirtAddr) -> Self {
        UserPtr {
            ptr: x as _,
            mark: PhantomData,
        }
    }
}

impl<T, P: Policy> UserPtr<T, P> {
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn add(&self, count: usize) -> Self {
        UserPtr {
            ptr: unsafe { self.ptr.add(count) },
            mark: PhantomData,
        }
    }

    unsafe fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    pub fn check(&self, count: usize) -> AcoreResult {
        if self.ptr.is_null() {
            return Err(AcoreError::Fault);
        }
        if (self.ptr as usize) % core::mem::align_of::<T>() != 0 {
            return Err(AcoreError::InvalidArgs);
        }
        if !user_access_ok(self.ptr as usize, core::mem::size_of::<T>() * count) {
            return Err(AcoreError::Fault);
        }
        Ok(())
    }
}

impl<T, P: Read> UserPtr<T, P> {
    pub fn read(&self) -> AcoreResult<T> {
        self.check(1)?;
        unsafe {
            let value = core::mem::MaybeUninit::uninit().assume_init();
            copy_from_user(&value as *const _ as *mut T, self.ptr, 1)?;
            Ok(value)
        }
    }

    pub fn read_if_not_null(&self) -> AcoreResult<Option<T>> {
        if self.ptr.is_null() {
            return Ok(None);
        }
        let value = self.read()?;
        Ok(Some(value))
    }

    pub fn read_array(&self, len: usize) -> AcoreResult<Vec<T>> {
        if len == 0 {
            return Ok(Vec::default());
        }
        self.check(len)?;
        let mut ret = Vec::<T>::with_capacity(len);
        unsafe {
            ret.set_len(len);
            copy_from_user(ret.as_mut_ptr(), self.ptr, len)?;
        }
        Ok(ret)
    }
}

impl<T, P: Write> UserPtr<T, P> {
    pub fn write(&mut self, value: T) -> AcoreResult {
        self.check(1)?;
        unsafe { copy_to_user(self.ptr, &value as *const T, 1)? };
        Ok(())
    }

    pub fn write_if_not_null(&mut self, value: T) -> AcoreResult {
        if self.ptr.is_null() {
            return Ok(());
        }
        self.write(value)
    }

    pub fn write_array(&mut self, values: &[T]) -> AcoreResult {
        if values.is_empty() {
            return Ok(());
        }
        self.check(values.len())?;
        unsafe { copy_to_user(self.ptr, values.as_ptr(), values.len())? };
        Ok(())
    }
}
