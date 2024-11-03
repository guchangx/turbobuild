
extern crate winapi;

mod detours;
mod hook;
mod utils;
mod replace;
mod functions;

use winapi::{
    shared::minwindef::{BOOL, DWORD, LPVOID, HINSTANCE}
};

static mut ENTRYPOINT: *mut std::ffi::c_void = 0 as _;

#[no_mangle]
unsafe extern "stdcall" fn DllMain(hinst_dll: HINSTANCE, fdw_reason: DWORD, lpv_reserved: LPVOID) -> BOOL {
    use std::os::windows::ffi::OsStrExt;

    if crate::detours::DetourIsHelperProcess() == winapi::shared::minwindef::TRUE {
        println!("DllMain is a helper process");
        //return true;
    }
    else {
        //println!("DllMain is a target process");
    }

    match fdw_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {

            unsafe {
                println!("message box for process attach");
                let text: Vec<u16> = std::ffi::OsStr::new("Debug BreakPoint")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let caption: Vec<u16> = std::ffi::OsStr::new("Attach Programe")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                winapi::um::winuser::MessageBoxW(0 as winapi::shared::windef::HWND, text.as_ptr(), caption.as_ptr(), 0);
            }

            //crate::detours::DetourRestoreAfterWithEx(pvData, cbData);
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
        
            ENTRYPOINT = crate::detours::DetourGetEntryPoint(std::ptr::null_mut());
        
            crate::detours::DetourAttach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);
        
            crate::hook::init_hook();
            
            let ret = crate::detours::DetourTransactionCommit();
            if ret != winapi::shared::winerror::NO_ERROR as i32 {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("DetourTransactionCommit: {}.", error_code);
            }
        },
        winapi::um::winnt::DLL_THREAD_ATTACH => {
            //println!("DLL_THREAD_ATTACH");
        },
        winapi::um::winnt::DLL_PROCESS_DETACH => {
            println!("DLL_PROCESS_DETACH");
            crate::detours::DetourTransactionBegin();
            crate::detours::DetourUpdateThread(winapi::um::processthreadsapi::GetCurrentThread() as _);
            //DetourDetach();
            crate::detours::DetourTransactionCommit();

        }
        winapi::um::winnt::DLL_THREAD_DETACH => {
            //println!("DLL_THREAD_DETACH");
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
}