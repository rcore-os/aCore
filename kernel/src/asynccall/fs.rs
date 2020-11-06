use super::AsyncCall;

impl AsyncCall {
    pub fn read(&self) {
        println!("READ {}", self.thread.id);
    }
}
