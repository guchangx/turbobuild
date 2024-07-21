
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
    
    unsafe {
        let title = crate::utils::convert::string_2_lpwstr("Debug BreakPoint".to_string());
        let caption = crate::utils::convert::string_2_lpwstr("Attarch Programe".to_string());
        winapi::um::winuser::MessageBoxW(0 as winapi::shared::windef::HWND, title.unwrap(), caption.unwrap(), 0);
    }

    if crate::detours::DetourIsHelperProcess() == winapi::shared::minwindef::TRUE {
        println!("DllMain is a helper process");
    }
    else {
        println!("DllMain is a target process");
    }

    match fdw_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {
            println!("DLL_PROCESS_ATTACH");
            println!("hinstDll: {:?}", hinst_dll);
            println!("lpvReserved: {:?}", lpv_reserved);
        },
        winapi::um::winnt::DLL_THREAD_ATTACH => {
            println!("DLL_THREAD_ATTACH");
            println!("hinstDll: {:?}", hinst_dll);
            println!("lpvReserved: {:?}", lpv_reserved);
        },
        winapi::um::winnt::DLL_PROCESS_DETACH => {
            println!("DLL_PROCESS_DETACH");
            println!("hinstDll: {:?}", hinst_dll);
            println!("lpvReserved: {:?}", lpv_reserved);
        }
        winapi::um::winnt::DLL_THREAD_DETACH => {

            println!("DLL_THREAD_DETACH");
            println!("hinstDll: {:?}", hinst_dll);
            println!("lpvReserved: {:?}", lpv_reserved);
        },
        _ => {
            println!("DllMain: unknown reason");
        },
    }


    let ret = crate::detours::DetourRestoreAfterWith();
    if ret == winapi::shared::minwindef::FALSE {
        let error_code = winapi::um::errhandlingapi::GetLastError();
        println!("DetourRestoreAfterWith failed, error code: {}.", error_code);
    }

    crate::detours::DetourTransactionBegin();
    
    let ret = crate::detours::DetourUpdateThread(winapi::um::processthreadsapi::GetCurrentThread() as _);
    if ret == winapi::shared::minwindef::FALSE {
        let error_code = winapi::um::errhandlingapi::GetLastError();
        println!("DetourUpdateThread failed, erro code: {}.", error_code);

    }

    ENTRYPOINT = crate::detours::DetourGetEntryPoint(std::ptr::null_mut());

    crate::detours::DetourAttach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);

    crate::hook::init_hook();
    
    crate::detours::DetourTransactionCommit();

    return 1;
}

unsafe fn main() {
    winapi::um::consoleapi::AllocConsole();
    println!("Process Hooked!");

    let start_discord: extern "C" fn() = std::mem::transmute(ENTRYPOINT);
    start_discord();
}