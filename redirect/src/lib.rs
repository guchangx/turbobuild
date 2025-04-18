
extern crate winapi;
mod detours;
mod hook;
mod utils;
mod replace;
mod functions;
mod ntdef;
mod logger;

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

static LOGGER: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex::<Option<tokio::sync::mpsc::Sender<String>>>>>
     = std::sync::LazyLock::new(|| {std::sync::Arc::new(std::sync::Mutex::new(None))});

static RUNTIME: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex::<tokio::runtime::Runtime>>> = std::sync::LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    std::sync::Arc::new(std::sync::Mutex::new(runtime))
});

/* 
static  PROJECTNAME: std::sync::LazyLock<std::sync::Mutex<Option<String>>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(None)
});
*/

static PROJECTNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/* 
static  REPLICADIR: std::sync::LazyLock<std::sync::Mutex<Option<String>>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(None)
});
*/

static REPLICADIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

unsafe fn redirect_stdout_log_2_cocrew() {

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(128);
    *LOGGER.lock().unwrap() = Some(tx.clone());

    use std::os::windows::ffi::OsStrExt;
    RUNTIME.lock().unwrap().spawn(async move {
        let name = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");
        let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    
        if  winapi::um::namedpipeapi::WaitNamedPipeW(name.as_ptr(), 100) == winapi::shared::minwindef::TRUE {
            let pipe = winapi::um::fileapi::CreateFileW(name.as_ptr(), 
                winapi::um::winnt::GENERIC_WRITE, 0,
                std::ptr::null_mut(),  
                winapi::um::fileapi::OPEN_EXISTING, 
                winapi::um::winnt::FILE_ATTRIBUTE_NORMAL, 
                winapi::shared::ntdef::NULL);
     
            if !pipe.is_null() && pipe != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let handle = tools::ptr::HandleBox::new(pipe);
    
                loop {
                    let message = rx.recv().await;
                    match message {
                        Some(message) => {

                            let mut bytes: winapi::shared::minwindef::DWORD = 0;
                            let result = winapi::um::fileapi::WriteFile(
                                handle.get().to_owned(),
                                message.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
                                message.len() as u32,
                                &mut bytes,
                                std::ptr::null_mut()
                            );
            
                            if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                                let error = winapi::um::errhandlingapi::GetLastError();
                                if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                    break;
                                }
                                println!("write pipe error, failed code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                                break;
                            }
                            unsafe { winapi::um::fileapi::FlushFileBuffers(handle.get().to_owned()) };
                        },
                        None => break,
                    }
                };
                winapi::um::namedpipeapi::DisconnectNamedPipe(handle.get().to_owned());
                winapi::um::handleapi::CloseHandle(handle.get().to_owned());
            }
        }
        else {
            let error = winapi::um::errhandlingapi::GetLastError();
            println!("WaitNamedPipeW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
        }
    });
}

fn read_project_property_from_stdin() {
    RUNTIME.lock().unwrap().spawn(
        async move {
        let stdin = std::io::stdin();
        let handle = stdin.lock();
        
        for line in handle.lines() {
            log!(trace, "read stdin pipe to string: {:?}", line);
            let arg = line.unwrap();
            if arg.starts_with("project") {
                //project:xxxxxxx or project xxxxxx 
                let (_, name) = arg.split_at("project".len() + 1);
                //*PROJECTNAME.lock().unwrap() = Some(name.to_string());
                PROJECTNAME.set(name.to_string()).unwrap();
            }
            else if arg.starts_with("replica") {
                let (_, dir) = arg.split_at("replica".len() + 1);
                //*REPLICADIR.lock().unwrap() = Some(dir.to_string());
                REPLICADIR.set(dir.to_string()).unwrap();
            }
        }
    });
}

#[macro_export]
macro_rules! log {
    ($level:ident, $($arg:tt)*) => {
        crate::logger::Logger::$level(format!("{}:{} {}", file!(), line!(), format!($($arg)*)))
    }
}

fn uninit_custom_resource() {
    if let Some(tx) = LOGGER.lock().unwrap().take() {
        drop(tx);
    }
}

use std::io::BufRead;

use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID};
#[no_mangle]
unsafe extern "stdcall" fn DllMain(_hinst: HINSTANCE, fdw_reason: DWORD, _reserved: LPVOID) -> BOOL {
    
    if crate::detours::DetourIsHelperProcess() == winapi::shared::minwindef::TRUE {
        //println!("target application is a helper process, so do nothing.");
        return winapi::shared::minwindef::TRUE;
    }

    match fdw_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {
            redirect_stdout_log_2_cocrew();
            read_project_property_from_stdin();
            
            //winapi::um::errhandlingapi::SetUnhandledExceptionFilter(Some(custom_exception_handler));
            /* 
            use std::os::windows::ffi::OsStrExt;
            let _text: Vec<u16> = std::ffi::OsStr::new("Debug BreakPoint")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let _caption: Vec<u16> = std::ffi::OsStr::new("Attach Programe")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            log!(info, "show debug message box for process attach");
            
            //just for attach debug
            //winapi::um::winuser::MessageBoxW(0 as winapi::shared::windef::HWND, text.as_ptr(), caption.as_ptr(), 0);
            */

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