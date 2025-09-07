use std::sync::{Condvar, Mutex};

pub struct Semaphore {
    max_threads: usize,
    mu: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads,
            mu: Mutex::new(0usize),
            cvar: Condvar::new(),
        }
    }

    pub fn lock(&self) {
        let mut count = self.mu.lock().unwrap();
        while *count >= self.max_threads {
            count = self.cvar.wait(count).unwrap();
        }
        *count += 1;
    }

    pub fn unlock(&self) {
        let mut count = self.mu.lock().unwrap();
        *count -= 1;
        self.cvar.notify_one();
    }
}
