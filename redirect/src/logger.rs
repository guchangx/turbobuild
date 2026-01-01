use windows_sys::Win32 as win;

pub fn output_debug_string(message: &String) {
    use std::os::windows::ffi::OsStrExt;
    let message = std::ffi::OsString::from(&message);
    let message = message.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    unsafe {
        win::System::Diagnostics::Debug::OutputDebugStringW(message.as_ptr());
    }
}

pub unsafe fn redirect_stdout_log_2_cocrew() {

    use std::os::windows::ffi::OsStrExt;
    let iocp = win::System::IO::CreateIoCompletionPort(
        win::Foundation::INVALID_HANDLE_VALUE,
        std::ptr::null_mut(),
        0,
        0
    );

    let iocp_handle = tools::ptr::HandleBox::new(iocp);
    let iocp_handle_ = iocp_handle.clone();

    let handle = crate::RUNTIME.lock().unwrap().spawn(async move {

        let name = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");
        let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    
        let mut rx = crate::LOGGER.rx.lock().unwrap().take().unwrap();

        if win::System::Pipes::WaitNamedPipeW(name.as_ptr(), 300) == win::Foundation::TRUE {

            for _ in 0..3 {
                let pipe_handle = win::Storage::FileSystem::CreateFileW(name.as_ptr(), win::Foundation::GENERIC_WRITE,
                    0,
                    std::ptr::null_mut(),  
                    win::Storage::FileSystem::OPEN_EXISTING, 
                    win::Storage::FileSystem::FILE_FLAG_OVERLAPPED, 
                    std::ptr::null_mut()
                );

                if !pipe_handle.is_null() && pipe_handle != win::Foundation::INVALID_HANDLE_VALUE {

                    let pipe_handle = tools::ptr::HandleBox::new(pipe_handle);

                    if win::System::IO::CreateIoCompletionPort(
                        pipe_handle.get().to_owned(),
                        iocp_handle.get().to_owned(),
                        0,
                        0
                    ).is_null() {
                        win::Foundation::CloseHandle(iocp_handle.get().to_owned());
                        win::Foundation::CloseHandle(pipe_handle.get().to_owned());
                        crate::logger::output_debug_string(&format!("redirect stdout CreateIoCompletionPort failed, error code: {}, message: {}", win::Foundation::GetLastError(), tools::utils::get_winapi_error_message(win::Foundation::GetLastError())));
                        break;
                    }

                    
                    loop {
                        let message = rx.recv().await;

                        match message {
                            Some(message) => {
                                let mut overlapped: win::System::IO::OVERLAPPED = std::mem::zeroed();
                                let mut bytes: u32 = 0;
                                let result = win::Storage::FileSystem::WriteFile(
                                    pipe_handle.get().to_owned(),
                                    message.as_bytes().as_ptr(),
                                    message.len() as u32,
                                    &mut bytes,
                                    &mut overlapped
                                );

                                if result == win::Foundation::FALSE {
                                    let error = win::Foundation::GetLastError();
                                    if error == win::Foundation::ERROR_IO_PENDING {
                                        
                                    }
                                    else if error == win::Foundation::ERROR_BROKEN_PIPE || error == win::Foundation::ERROR_NO_DATA {
                                        //break;
                                    }
                                    else {
                                        crate::logger::output_debug_string(&format!("write pipe error, failed code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
                                        break;
                                    }
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    };
                    
                    win::System::IO::PostQueuedCompletionStatus(
                        iocp_handle.get().to_owned(), 
                        0, 
                        0, 
                        std::ptr::null_mut()
                    );

                    win::Foundation::CloseHandle(pipe_handle.get().to_owned());
                    break;
                }
                else {
                    let error = win::Foundation::GetLastError();
                    
                    if error == win::Foundation::ERROR_PIPE_BUSY || error == win::Foundation::ERROR_FILE_NOT_FOUND {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                    else {
                        crate::logger::output_debug_string(&format!("redirect stdout createFileW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
                    }
                }
            }
        }
        else {
            let error = win::Foundation::GetLastError();
            //i do not know why println!() call case cl.exe stdoutput. so use output_debug_string. 
            crate::logger::output_debug_string(&format!("redirect stdout WaitNamedPipeW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
        }
        
        //drain remaining messages
        loop {
            match rx.try_recv() {
                Ok(_) => {
                },
                Err(err) => {
                    crate::logger::output_debug_string(&format!("redirect_stdout_log_2_cocrew: failed to receive message: {}", err));
                    break;
                },
            }
        }
        rx.close();
        
        return ();
    });

    let _ = std::thread::spawn(move || {
        crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus start."));
        loop {
            let mut bytes: u32 = 0;
            let mut key: usize = 0;
            let mut overlapped: *mut win::System::IO::OVERLAPPED = std::ptr::null_mut();

            let result = win::System::IO::GetQueuedCompletionStatus(
                iocp_handle_.get().to_owned(),
                &mut bytes,
                &mut key,
                &mut overlapped,
                win::System::Threading::INFINITE
            );

            if result == win::Foundation::FALSE || bytes == 0 {
                let error = win::Foundation::GetLastError();
                crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
                break;
            }
            else {
                if key == 0 {
                    break;
                }
            }
        }
        win::Foundation::CloseHandle(iocp_handle_.get().to_owned());
        crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus end."));
    });
}


static LOGGER_START_TIME: std::sync::LazyLock<chrono::DateTime<chrono::Local>> = std::sync::LazyLock::new(|| chrono::Local::now());
static LOGGER_START_INSTANT: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(|| std::time::Instant::now());

pub struct Logger;

impl Logger {

    #[inline(always)]
    pub fn log(message: impl Into<String>) {
        let message = message.into();

        //output_debug_string(&message);

        match crate::LOGGER.tx.try_send(message) {
            Ok(_) => {},
            Err(_) => {
                //println!("logger send message failed: {}, message: {:?} capacity: {:?}", e, message, crate::LOGGER.tx.capacity());
            }
        }
    }

    #[inline(always)]
    pub fn trace(message: impl Into<String>) {
        let now = *LOGGER_START_TIME + LOGGER_START_INSTANT.elapsed();
        let time = now.format("%H:%M:%S%.3f").to_string();

        Self::log(format!("[{} T] [{}] {}", time, *crate::PROCESS_ID, message.into()));
    }

    #[inline(always)]
    pub fn debug(message: impl Into<String>) {
        let now = *LOGGER_START_TIME + LOGGER_START_INSTANT.elapsed();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} D] [{}] {}", time, *crate::PROCESS_ID, message.into()));
    }

    #[inline(always)]
    pub fn info(message: impl Into<String>) {
        let now = *LOGGER_START_TIME + LOGGER_START_INSTANT.elapsed();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} I] [{}] {}", time, *crate::PROCESS_ID, message.into()));
    }

    #[inline(always)]
    pub fn warn(message: impl Into<String>) {
        let now = *LOGGER_START_TIME + LOGGER_START_INSTANT.elapsed();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} W] [{}] {}", time, *crate::PROCESS_ID, message.into()));
    }

    #[inline(always)]
    pub fn error(message: impl Into<String>) {
        let now = *LOGGER_START_TIME + LOGGER_START_INSTANT.elapsed();
        let time = now.format("%H:%M:%S%.3f").to_string();
        Self::log(format!("[{} E] [{}] {}", time, *crate::PROCESS_ID, message.into()));
    }

}

pub static LOGGER_LEVEL: LogLevel = LogLevel::Trace;

#[derive(PartialOrd, PartialEq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[macro_export]
macro_rules! log {
    ($level:ident, $($arg:tt)*) => {
        {   

            let level = match stringify!($level) {
                "trace" => crate::logger::LogLevel::Trace,
                "debug" => crate::logger::LogLevel::Debug,
                "info" => crate::logger::LogLevel::Info,
                "warn" => crate::logger::LogLevel::Warn,
                "error" => crate::logger::LogLevel::Error,
                _ => crate::logger::LogLevel::Warn,
            };

            if level >= crate::logger::LOGGER_LEVEL {
                let message = format!("{}:{} {}", file!().split(r"\").last().unwrap_or("<unnamed>"), line!(), format!($($arg)*));
                crate::logger::Logger::$level(message);
            }
        }
    };
}