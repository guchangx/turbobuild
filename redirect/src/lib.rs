
extern crate winapi;

mod detours;
mod hook;
mod utils;
mod replace;
mod functions;
mod ntdef;
mod logger;

use winapi::{
    shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID}, um::handleapi::CloseHandle
};


static mut ENTRYPOINT: *mut std::ffi::c_void = 0 as _;

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

unsafe fn redirect_stdout_log_2_cocrew() {
    use std::os::windows::ffi::OsStrExt;
    let name = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");
    let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

    if  winapi::um::namedpipeapi::WaitNamedPipeW(name.as_ptr(), 0) == winapi::shared::minwindef::TRUE {
        
        let pipe = winapi::um::fileapi::CreateFileW(name.as_ptr(), 
            winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE, 0,
		    std::ptr::null_mut(),  winapi::um::fileapi::OPEN_EXISTING, 
            winapi::um::winnt::FILE_ATTRIBUTE_NORMAL, 
            winapi::shared::ntdef::NULL);
 
        if !pipe.is_null() {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1024);
            *LOGGER.lock().unwrap() = Some(tx.clone());

            let handle = tools::ptr::HandleBox::new(pipe);
            let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
            runtime.spawn(async move {
                loop {
                    let message = rx.recv().await;
                    match message {
                        Some(message) => {

                            let buffer = message + "\r\n";
    
                            let mut bytes: winapi::shared::minwindef::DWORD = 0;
                            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
                            let result = winapi::um::fileapi::WriteFile(
                                handle.get().to_owned(),
                                buffer.as_bytes().as_ptr() as *const std::ffi::c_void,
                                buffer.len() as u32,
                                &mut bytes,
                                &mut overlapped
                            );
            
                            if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                                let error = winapi::um::errhandlingapi::GetLastError();
                                if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                    break;
                                }
                                println!("write pipe error, failed code: {}", error);
                                break;
                            }
                        },
                        None => break,
                    }
                }
            });

            winapi::um::namedpipeapi::DisconnectNamedPipe(pipe);
            CloseHandle(pipe);
        }
    }
}

fn uninit_custom_resource() {
    if let Some(tx) = LOGGER.lock().unwrap().take() {
        drop(tx);
    }
}

#[no_mangle]
unsafe extern "stdcall" fn DllMain(_hinst: HINSTANCE, fdw_reason: DWORD, _reserved: LPVOID) -> BOOL {
    use std::os::windows::ffi::OsStrExt;

    if crate::detours::DetourIsHelperProcess() == winapi::shared::minwindef::TRUE {
        println!("target application is a helper process, so do nothing.");
        return winapi::shared::minwindef::TRUE;
    }

    match fdw_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {
	       
            redirect_stdout_log_2_cocrew();

            if let Some(tx) = LOGGER.lock().unwrap().as_ref() {
                let _ = tx.try_send("DLL_PROCESS_ATTACH".to_string());
            }

            winapi::um::consoleapi::AllocConsole();

            winapi::um::errhandlingapi::SetUnhandledExceptionFilter(Some(custom_exception_handler));
    
            println!("message box for process attach");

            logger::Logger::info(format!("{}{}{} message box for process attach", module_path!(), file!(), line!()));

            let text: Vec<u16> = std::ffi::OsStr::new("Debug BreakPoint")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let caption: Vec<u16> = std::ffi::OsStr::new("Attach Programe")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            winapi::um::winuser::MessageBoxW(0 as winapi::shared::windef::HWND, text.as_ptr(), caption.as_ptr(), 0);
            //just for attach debug
            

            let ret = crate::detours::DetourRestoreAfterWith();
            if ret == winapi::shared::minwindef::FALSE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("DetourRestoreAfterWith failed, error code: {}.", error_code);
            }

            let ret = crate::detours::DetourTransactionBegin();
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("DetourTransactionBegin failed, erro code: {}.", error_code);
            }

            let ret = crate::detours::DetourUpdateThread(winapi::um::processthreadsapi::GetCurrentThread() as _);
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("DetourUpdateThread failed, erro code: {}.", error_code);
            }
        
            //ENTRYPOINT = crate::detours::DetourGetEntryPoint(std::ptr::null_mut());
            //crate::detours::DetourAttach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);
            //hook entry point

            crate::hook::init_hook();
            
            let ret = crate::detours::DetourTransactionCommit();
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("DetourTransactionCommit: {}.", error_code);
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
            println!("DLL_PROCESS_DETACH");
        }
        winapi::um::winnt::DLL_THREAD_DETACH => {

        },
        _ => {
            println!("DllMain: unknown reason");
        },
    }

    return 1;

}

unsafe fn main() {
    winapi::um::consoleapi::AllocConsole();
    println!("redirect process hooked!");
    let start_redirect: extern "C" fn() = std::mem::transmute(ENTRYPOINT);
    start_redirect();
    println!("redirect process end!");
    winapi::um::wincon::FreeConsole();
}