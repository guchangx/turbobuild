
extern crate winapi;
mod detours;
mod hook;
mod utils;
mod replace;
mod functions;
mod ntdef;
mod logger;
pub mod netredirect;

//TODO The current size of the package is 1.24M
//TODO Remove winapi, use windows-sys replace and remove ntdef.

unsafe extern "system" fn custom_exception_handler(
    exception_info: *mut winapi::um::winnt::EXCEPTION_POINTERS
) -> i32 {
    let exception_record = (*exception_info).ExceptionRecord;
    if !exception_record.is_null() {
        let code = (*exception_record).ExceptionCode;

        if code == 0x80000003 {  // EXCEPTION_BREAKPOINT
            println!("Handling DbgBreakPoint exception");
            return -1; // EXCEPTION_CONTINUE_EXECUTION
        }
    }
    winapi::um::errhandlingapi::UnhandledExceptionFilter(exception_info)
}
struct Channel {
    tx: tokio::sync::mpsc::Sender<String>, 
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<String>>>,
}

static LOGGER: std::sync::LazyLock<Channel> = std::sync::LazyLock::new(|| {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(1024);
        let channel = Channel { tx, rx: std::sync::Mutex::new(Some(rx)) };
        return channel; 
});

static RUNTIME: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex::<tokio::runtime::Runtime>>> = std::sync::LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    std::sync::Arc::new(std::sync::Mutex::new(runtime))
});

/* 
static  PROJECTNAME: std::sync::LazyLock<std::sync::Mutex<Option<String>>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(None)
});
*/

//from buildassist read command line up to the present moment elapsed time is about 1.5s.
static SOLUTIONNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static PROJECTNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/* 
static  REPLICADIR: std::sync::LazyLock<std::sync::Mutex<Option<String>>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(None)
});
*/

static REPLICADIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static GENERATEDDIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static INCLUDES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
static STDOUT_LOG_HANDLE: std::sync::LazyLock<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));
static WORKINGDIR: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    if let Ok(path) = std::env::current_dir() {
        path.to_string_lossy().to_string()
    } else {
        String::from(".")
    }
});

unsafe fn redirect_stdout_log_2_cocrew() {

    use std::os::windows::ffi::OsStrExt;
    let iocp = winapi::um::ioapiset::CreateIoCompletionPort(
        winapi::um::handleapi::INVALID_HANDLE_VALUE,
        std::ptr::null_mut(),
        0,
        0
    );

    let iocp_handle = tools::ptr::HandleBox::new(iocp);
    let iocp_handle_ = iocp_handle.clone();

    let handle = RUNTIME.lock().unwrap().spawn(async move {

        let name = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");
        let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    
        let mut rx = crate::LOGGER.rx.lock().unwrap().take().unwrap();

        if winapi::um::namedpipeapi::WaitNamedPipeW(name.as_ptr(), 300) == winapi::shared::minwindef::TRUE {

            for _ in 0..3 {
                let pipe_handle = winapi::um::fileapi::CreateFileW(name.as_ptr(), winapi::um::winnt::GENERIC_WRITE,
                    0,
                    std::ptr::null_mut(),  
                    winapi::um::fileapi::OPEN_EXISTING, 
                    winapi::um::winbase::FILE_FLAG_OVERLAPPED, 
                    winapi::shared::ntdef::NULL
                );

                if !pipe_handle.is_null() && pipe_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {

                    let pipe_handle = tools::ptr::HandleBox::new(pipe_handle);

                    if winapi::um::ioapiset::CreateIoCompletionPort(
                        pipe_handle.get().to_owned(),
                        iocp_handle.get().to_owned(),
                        0,
                        0
                    ).is_null() {
                        winapi::um::handleapi::CloseHandle(iocp_handle.get().to_owned());
                        winapi::um::handleapi::CloseHandle(pipe_handle.get().to_owned());
                        crate::logger::output_debug_string(&format!("redirect stdout CreateIoCompletionPort failed, error code: {}, message: {}", winapi::um::errhandlingapi::GetLastError(), tools::utils::get_winapi_error_message(winapi::um::errhandlingapi::GetLastError())));
                        break;
                    }

                    loop {
                        let message = rx.recv().await;

                        match message {
                            Some(message) => {
                                let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
                                let mut bytes: winapi::shared::minwindef::DWORD = 0;
                                let result = winapi::um::fileapi::WriteFile(
                                    pipe_handle.get().to_owned(),
                                    message.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
                                    message.len() as u32,
                                    &mut bytes,
                                    &mut overlapped
                                );

                                if result == winapi::shared::minwindef::FALSE {
                                    let error = winapi::um::errhandlingapi::GetLastError();
                                    if error == winapi::shared::winerror::ERROR_IO_PENDING {
                                        
                                    }
                                    else if error == winapi::shared::winerror::ERROR_BROKEN_PIPE || error == winapi::shared::winerror::ERROR_NO_DATA {
                                        //break;
                                    }
                                    else {
                                        crate::logger::output_debug_string(&format!("write pipe error, failed code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
                                        break;
                                    }
                                }
                                
                                //if winapi::shared::minwindef::FALSE == winapi::um::fileapi::FlushFileBuffers(pipe_handle.get().to_owned()) {
                                //    println!("FlushFileBuffers failed, error code: {}, message: {}", winapi::um::errhandlingapi::GetLastError(), tools::utils::get_winapi_error_message(winapi::um::errhandlingapi::GetLastError()));
                                //}
                            },
                            None => {
                                break;
                            }
                        }
                    };
                    
                    winapi::um::ioapiset::PostQueuedCompletionStatus(
                        iocp_handle.get().to_owned(), 
                        0, 
                        0, 
                        std::ptr::null_mut()
                    );

                    winapi::um::handleapi::CloseHandle(pipe_handle.get().to_owned());
                    break;
                }
                else {
                    let error = winapi::um::errhandlingapi::GetLastError();
                    
                    if error == winapi::shared::winerror::ERROR_PIPE_BUSY || error == winapi::shared::winerror::ERROR_FILE_NOT_FOUND {
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
            let error = winapi::um::errhandlingapi::GetLastError();
            //i do not know why println!() call case cl.exe stdoutput. so use output_debug_string. 
            crate::logger::output_debug_string(&format!("redirect stdout WaitNamedPipeW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
        }
        
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

    *STDOUT_LOG_HANDLE.lock().unwrap() = Some(handle);

    let _ = std::thread::spawn(move || {
        crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus start."));
        loop {
            let mut bytes: winapi::shared::minwindef::DWORD = 0;
            let mut key: usize = 0;
            let mut overlapped: *mut winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();

            let result = winapi::um::ioapiset::GetQueuedCompletionStatus(
                iocp_handle_.get().to_owned(),
                &mut bytes,
                &mut key,
                &mut overlapped,
                winapi::um::winbase::INFINITE
            );

            if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                let error = winapi::um::errhandlingapi::GetLastError();
                crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
                break;
            }
            else {
                if key == 0 {
                    break;
                }
            }
        }
        winapi::um::handleapi::CloseHandle(iocp_handle_.get().to_owned());
        crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus end."));
    });
}

fn read_project_property_from_stdin() {
    //let hendle = RUNTIME.lock().unwrap().spawn(
    //    async move {
        let stdin = std::io::stdin();
        let handle = stdin.lock();
        
        for line in handle.lines() {
            log!(trace, "read stdin pipe to string: {:?}", line);
            let arg = line.unwrap();
            if arg.starts_with("solution") {
                //solution:xxxxxxx or solution xxxxxx
                let (_, sln) = arg.split_at("solution".len() + 1);
                SOLUTIONNAME.set(String::from(&sln[0..sln.len()])).unwrap();
            }
            else if arg.starts_with("project") {
                //project:xxxxxxx or project xxxxxx 
                let (_, proj) = arg.split_at("project".len() + 1);
                PROJECTNAME.set(String::from(&proj[0..proj.len()])).unwrap();
            }
            else if arg.starts_with("replica") {
                let (_, dir) = arg.split_at("replica".len() + 1);
                REPLICADIR.set(String::from(&dir[0..dir.len()])).unwrap();
            }
        }
    //});
}

static MODULE_PATH: std::sync::OnceLock<std::ffi::CString> = std::sync::OnceLock::new();

fn fetch_module_path(hinst: HINSTANCE) {
    let mut buffer = vec![0u16; 512];
    let length = unsafe { GetModuleFileNameW(hinst, buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 {
        log!(error, "GetModuleFileNameW failed, error code: {}", unsafe {winapi::um::errhandlingapi::GetLastError()});
    }
    else {
        use std::os::windows::ffi::OsStringExt;
        buffer.truncate(length as usize);
        let path = std::ffi::OsString::from_wide(&buffer);
        let path = std::ffi::CString::new(path.to_string_lossy().to_string()).unwrap();
        log!(debug, "current module path: {:?}", path);
        MODULE_PATH.set(path).unwrap();
    }
}

fn fetch_args_from_command() {
    let commandline = std::env::args_os();
    let commands: Vec<std::ffi::OsString> = commandline.collect();
    log!(debug, "current command line arguments: {:?}", commands);
    let mut includes = Vec::new();
    let mut sources_dir = String::new();

    for (index, item) in commands.iter().enumerate() {

        let item = item.to_string_lossy();

        if item.eq("/I") || item.eq("/external:I") {
            if index + 1 < commands.len() {
                let include = commands[index + 1].to_string_lossy().to_string();
                includes.push(include);
            }
        }
        else if item.starts_with("/Fo") {
            if item.ends_with(".obj") {
                std::path::Path::new(&item[3..]).parent().map(|parent| {
                    GENERATEDDIR.set(parent.to_string_lossy().to_string()).unwrap();
                });
            }
            else {
                GENERATEDDIR.set(item[3..].to_string()).unwrap();
            }
        }
        else if item.starts_with("/Fd") {
            if item.ends_with(".pdb") {
                std::path::Path::new(&item[3..]).parent().map(|parent| {
                    GENERATEDDIR.get_or_init(|| parent.to_string_lossy().to_string());
                });
            }
            else {
                GENERATEDDIR.get_or_init(|| item[3..].to_string());
            }
        }
        else if sources_dir.is_empty() && (item.ends_with(".c") || item.ends_with(".cpp") || item.ends_with(".cc") || item.ends_with(".cxx")) {
            if let Some(dir) = std::path::Path::new(&item.to_string()).parent() {
                sources_dir = dir.to_string_lossy().to_string();
            };
        }
    }
    
    includes.push(sources_dir);

    for (key, value) in std::env::vars() {
        if key.to_lowercase() == "include" {
            let paths: Vec<&str> = value.split(';').collect();
            for path in paths {
                if !path.is_empty() {
                    includes.push(path.to_string());
                }
            }
        }
    }
    INCLUDES.set(includes).unwrap();

}   

static LOGGER_LEVEL: LogLevel = LogLevel::Trace;

#[derive(PartialOrd, PartialEq)]
enum LogLevel {
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
                "trace" => crate::LogLevel::Trace,
                "debug" => crate::LogLevel::Debug,
                "info" => crate::LogLevel::Info,
                "warn" => crate::LogLevel::Warn,
                "error" => crate::LogLevel::Error,
                _ => crate::LogLevel::Warn,
            };

            if level >= crate::LOGGER_LEVEL {
                let message = format!("{}:{} {}", file!().split(r"\").last().unwrap_or("<unnamed>"), line!(), format!($($arg)*));
                crate::logger::Logger::$level(message.clone());
            }
        }
    };
}

fn uninit_custom_resource() {
    log!(info, "[{:?}] uninit custom resource. receive is closed: {}", crate::PROJECTNAME.get(), LOGGER.tx.capacity());
}

use std::io::BufRead;

use winapi::{shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID}, um::libloaderapi::GetModuleFileNameW};

unsafe fn show_message_box_for_debug() {
    use std::os::windows::ffi::OsStrExt;
    let text: Vec<u16> = std::ffi::OsStr::new("Debug BreakPoint")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let caption: Vec<u16> = std::ffi::OsStr::new("Attach Programe")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    log!(info, "show debug message box for process attach");
    
    //just for attach debug, when message box block process run.
    winapi::um::winuser::MessageBoxW(0 as winapi::shared::windef::HWND, text.as_ptr(), caption.as_ptr(), 0);
}

thread_local! {
    static IN_HOOK: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[no_mangle]
unsafe extern "stdcall" fn DllMain(hinst: HINSTANCE, fdw_reason: DWORD, _reserved: LPVOID) -> BOOL {
    
    if crate::detours::DetourIsHelperProcess() == winapi::shared::minwindef::TRUE {
        //println!("target application is a helper process, so do nothing.");
        return winapi::shared::minwindef::TRUE;
    }

    match fdw_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {

            //force_unbuffered_output();

            fetch_module_path(hinst);
            fetch_args_from_command(); 

            redirect_stdout_log_2_cocrew();
            read_project_property_from_stdin();
            
            crate::netredirect::async_connect_named_pipe();

            //show_message_box_for_debug();

            let ret = crate::detours::DetourRestoreAfterWith();
            if ret == winapi::shared::minwindef::FALSE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                log!(error, "DetourRestoreAfterWith failed, error code: {}.", error_code);
            }

            let ret = crate::detours::DetourTransactionBegin();
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                log!(error, "DetourTransactionBegin failed, error code: {}.", error_code);
            }

            let ret = crate::detours::DetourUpdateThread(winapi::um::processthreadsapi::GetCurrentThread() as _);
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                log!(error, "DetourUpdateThread failed, erro code: {}.", error_code);
            }
        
            //ENTRYPOINT = crate::detours::DetourGetEntryPoint(std::ptr::null_mut());
            //crate::detours::DetourAttach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);
            //hook entry point

            crate::hook::init_hook();
            
            let ret = crate::detours::DetourTransactionCommit();
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                log!(error, "DetourTransactionCommit failed, error code: {}.", error_code);
            }
        },
        winapi::um::winnt::DLL_THREAD_ATTACH => {

        },
        winapi::um::winnt::DLL_PROCESS_DETACH => {
            crate::detours::DetourTransactionBegin();
            crate::detours::DetourUpdateThread(winapi::um::processthreadsapi::GetCurrentThread() as _);
           
            //DetourDetach();

            //crate::detours::DetourDetach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);
            //unhook entry point
            crate::detours::DetourTransactionCommit();

            uninit_custom_resource();
        }
        winapi::um::winnt::DLL_THREAD_DETACH => {

        },
        _ => {
        },
    }

    return winapi::shared::minwindef::TRUE;
}


extern "C" {
    fn setvbuf(stream: *mut std::ffi::c_void, buffer: *mut std::ffi::c_char, mode: std::ffi::c_int, size: usize) -> std::ffi::c_int;
    fn __acrt_iob_func(index: std::ffi::c_uint) -> *mut std::ffi::c_void;
    fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
}


const _IOFBF: std::ffi::c_int = 0; 
const _IOLBF: std::ffi::c_int = 1; 
const _IONBF: std::ffi::c_int = 2; 


unsafe fn force_unbuffered_output() {
    
    let stdout_ptr = __acrt_iob_func(1);
    let stderr_ptr = __acrt_iob_func(2);
    
    if !stdout_ptr.is_null() && !stderr_ptr.is_null() {

        let _result_stdout = setvbuf(stdout_ptr, std::ptr::null_mut(), _IOLBF, 0);
        let _result_stdout = setvbuf(stderr_ptr, std::ptr::null_mut(), _IOLBF, 0);
        

        // let result1 = setvbuf(stdout_ptr, std::ptr::null_mut(), _IONBF, 0);
        // let result2 = setvbuf(stderr_ptr, std::ptr::null_mut(), _IONBF, 0);


        fflush(stdout_ptr);
        fflush(stderr_ptr);
    } else {
        crate::log!(warn, "Failed to get stdout/stderr pointers");
    }
}