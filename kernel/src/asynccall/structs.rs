#[repr(C)]
#[derive(Debug)]
pub struct AsyncCallBuffer {
    pub data: [u8; 23],
}

impl AsyncCallBuffer {
    pub fn new() -> Self {
        Self { data: [b'a'; 23] }
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self as *const _ as *const T
    }
}
