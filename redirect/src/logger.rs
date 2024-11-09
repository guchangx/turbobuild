pub struct Logger;

impl Logger {
    pub fn log(message: impl Into<String>) {
        if let Some(tx) = crate::LOGGER.lock().unwrap().as_ref() {
            let _ = tx.try_send(message.into());
        }
    }

    pub fn trace(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        Self::log(format!("[{} TRACE redriect] {}",time, message.into()));
    }
    pub fn debug(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        Self::log(format!("[{} DEBUG] {}", time, message.into()));
    }
    pub fn info(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        Self::log(format!("[{} INFO] {}", time, message.into()));
    }
    pub fn warn(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        Self::log(format!("[{} WARN] {}", time, message.into()));
    }
    pub fn error(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        Self::log(format!("[{} ERROR] {}", time, message.into()));
    }


}