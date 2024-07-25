
use winapi::um::processthreadsapi::CreateProcessW;

use crate::detours::detours::DetourCreateProcessWithDllExW;
use std::os::windows::ffi::OsStrExt;


pub fn msvc_detours(app_path: String, command_line: String, workding_directory: String) {
    println!("msvc detours");
    unsafe {
        //let lpApplicationName = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe";
        let lpApplicationName =  app_path.as_str();
        
        //let lpCommandLine = "-c test.c";
        let lpCommandLine = command_line.as_str();
        //let lpProcessAttributes = std::ptr::null_mut();
        //let lpThreadAttributes = std::ptr::null_mut();
        let bInheritHandles = 0;   
        let dwCreationFlags = winapi::um::winbase::CREATE_DEFAULT_ERROR_MODE | winapi::um::winbase::CREATE_SUSPENDED;
        let lpEnvironment = std::ptr::null_mut();

        //let mut lpStartupInfo = std::mem::MaybeUninit::zeroed().assume_init(); 
        //let mut lpProcessInformation = std::mem::MaybeUninit::zeroed().assume_init();
        
        let appName = std::ffi::OsStr::new(lpApplicationName);
        let appNameWideChars: Vec<u16> = appName.encode_wide().chain(std::iter::once(0)).collect();

        let commandLine = std::ffi::OsStr::new(lpCommandLine);
        let mut commandLineWideChars: Vec<u16> = commandLine.encode_wide().chain(std::iter::once(0)).collect();
        
        let lpCurrentDirectory = std::ffi::OsStr::new(&workding_directory);
        let currentDirectoryWideChars: Vec<u16> = lpCurrentDirectory.encode_wide().chain(std::iter::once(0)).collect();
 
        let dllPath = crate::utils::tool::get_working_path("redirect64.dll".to_string());
        if let Some(dllPath) = dllPath {

            let dllPath = std::ffi::CString::new(dllPath).unwrap();
            let dllPath = dllPath.as_ptr();

                    
            /* 
            let ret = CreateProcessW(appNameWideChars.as_ptr(), 
                commandLineWideChars.as_mut_ptr(), 
                lpProcessAttributes, 
                lpThreadAttributes, 
                bInheritHandles, 
                dwCreationFlags, 
                lpEnvironment, 
                currentDirectoryWideChars.as_ptr(), 
                &mut lpStartupInfo as *mut _, 
                &mut lpProcessInformation as *mut _);

            if ret == winapi::shared::minwindef::TRUE
            {
                println!("CreateProcessW success!");
            }
            else {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("CreateProcessW failed! error_code: {}.", error_code);
            }
            */
        
            let lpProcessAttributes = std::ptr::null_mut();
            let lpThreadAttributes = std::ptr::null_mut();
            let mut lpStartupInfo: crate::detours::detours::_STARTUPINFOW = std::mem::MaybeUninit::zeroed().assume_init(); 
            let mut lpProcessInformation: crate::detours::detours::_PROCESS_INFORMATION = std::mem::MaybeUninit::zeroed().assume_init();

            let ret = DetourCreateProcessWithDllExW(appNameWideChars.as_ptr(), 
                commandLineWideChars.as_mut_ptr(), 
                lpProcessAttributes,
                lpThreadAttributes, 
                bInheritHandles, dwCreationFlags, 
                lpEnvironment, 
                currentDirectoryWideChars.as_ptr(), 
                &mut lpStartupInfo as *mut _, 
                &mut lpProcessInformation as *mut _, 
                dllPath, 
                Option::None);
            
            if ret == winapi::shared::minwindef::TRUE
            {
                println!("DetourCreateProcessWithDllExW success!");
            }
            else {
                let error_code: u32 = winapi::um::errhandlingapi::GetLastError();
                println!("DetourCreateProcessWithDllExW failed! error_code: {}.", error_code);
            }

            let ret = winapi::um::processthreadsapi::ResumeThread(lpProcessInformation.hThread as _);
            if ret == winapi::shared::minwindef::TRUE as u32
            {
                println!("ResumeThread success!");
            }
            else {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                println!("ResumeThread failed! error_code: {}.", error_code);
            }

            winapi::um::handleapi::CloseHandle(lpProcessInformation.hThread as _);
            winapi::um::handleapi::CloseHandle(lpProcessInformation.hProcess as _);
            
            println!("msvc detours end");
        }
    }
} 