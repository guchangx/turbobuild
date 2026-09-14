use std::sync::{Arc, Mutex, Condvar};

#[derive(Clone)]
pub struct BlockingSlot<T> {
    inner: Arc<(Mutex<Option<T>>, Condvar)>,
}

impl<T> BlockingSlot<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    pub fn write(&self, value: T) {
        let (lock, cvar) = &*self.inner;
        let mut data = lock.lock().unwrap();
        *data = Some(value);
        cvar.notify_one();
    }

    pub fn blocking_read(&self) -> T {
        let (lock, cvar) = &*self.inner;
        let mut data = lock.lock().unwrap();
        loop {
 
            if let Some(val) = data.take() {
                return val;
            }
            data = cvar.wait(data).unwrap();
        }
    }
}