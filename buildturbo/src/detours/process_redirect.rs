#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::detours::detours::DetourCreateProcessWithDllExW;
use std::os::windows::ffi::OsStrExt;


fn msvc_detours(app_name: String, command_line: String, workding_directory: String) {
    unsafe {

        let lpApplicationName = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe";
        let lpCommandLine = "-c test.c";
        let lpProcessAttributes = std::ptr::null_mut();
        let lpThreadAttributes = std::ptr::null_mut();
        let bInheritHandles = 0;   
        let dwCreationFlags = winapi::um::winbase::CREATE_DEFAULT_ERROR_MODE | winapi::um::winbase::CREATE_SUSPENDED;
        let lpEnvironment = std::ptr::null_mut();

        let mut lpStartupInfo: crate::detours::detours::_STARTUPINFOW = std::mem::MaybeUninit::zeroed().assume_init(); 
        let mut lpProcessInformation: crate::detours::detours::_PROCESS_INFORMATION = std::mem::MaybeUninit::zeroed().assume_init();
        
        let appName = std::ffi::OsStr::new(lpApplicationName);
        let appNameWideChars: Vec<u16> = appName.encode_wide().chain(std::iter::once(0)).collect();
        //let const_u16_ptr: *const u16 = appNameWideChars.as_ptr();

        let commandLine = std::ffi::OsStr::new(lpCommandLine);
        let mut commandLineWideChars: Vec<u16> = commandLine.encode_wide().chain(std::iter::once(0)).collect();
        
        let lpCurrentDirectory = std::ffi::OsStr::new(&workding_directory);
        let currentDirectoryWideChars: Vec<u16> = lpCurrentDirectory.encode_wide().chain(std::iter::once(0)).collect();

        let dllPath = std::ffi::CString::new("").unwrap();
        let dllPath = dllPath.as_ptr();
        

        let ret = DetourCreateProcessWithDllExW(appNameWideChars.as_ptr(), commandLineWideChars.as_mut_ptr(), 
            lpProcessAttributes, lpThreadAttributes, 
            bInheritHandles, dwCreationFlags, 
            lpEnvironment, currentDirectoryWideChars.as_ptr(), 
            &mut lpStartupInfo as *mut _, &mut lpProcessInformation as *mut _, 
            dllPath, Option::None);
        
        if ret == 0
        {
            println!("DetourCreateProcessWithDllExW success!");
        }
        else {
            let error_code = winapi::um::errhandlingapi::GetLastError();
            println!("DetourCreateProcessWithDllExW failed! error_code: {}.", error_code);
        }
    }
} 