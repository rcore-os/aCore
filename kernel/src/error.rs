#![allow(dead_code)]

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AcoreError {
    Ok = 0,
    Internel = -1,
    NotSupported = -2,
    NoMemory = -3,
    InvalidArgs = -4,
    OutOfRange = -5,
    BadState = -6,
    NotFound = -7,
    AlreadyExists = -8,
    AccessDenied = -9,
}

pub type AcoreResult<T = ()> = Result<T, AcoreError>;
