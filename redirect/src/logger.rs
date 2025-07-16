
pub fn output_debug_string(message: &String) {
    use std::os::windows::ffi::OsStrExt;
    let message = std::ffi::OsString::from(message);
    let message = message.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    unsafe {
        winapi::um::debugapi::OutputDebugStringW(message.as_ptr());
    }
}

pub struct Logger;

impl Logger {
    pub fn log(message: impl Into<String>) {
        let message = message.into();

        output_debug_string(&message);

        match crate::LOGGER.tx.try_send(message.clone()) {
            Ok(_) => {},
            Err(e) => {
                println!("logger send message failed: {}, message: {:?} capacity: {:?}", e, message, crate::LOGGER.tx.capacity());
            }
        }
    }

    pub fn trace(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} T] {}",time, message.into()));
    }
    pub fn debug(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} D] {}", time, message.into()));
    }
    pub fn info(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} I] {}", time, message.into()));
    }
    pub fn warn(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} W] {}", time, message.into()));
    }
    pub fn error(message: impl Into<String>) {
        let now = chrono::Local::now();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} E] {}", time, message.into()));
    }


}