
use winapi::um::{handleapi::CloseHandle, processthreadsapi::CreateProcessW};

use crate::detours::detours::DetourCreateProcessWithDllExW;
use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};

pub fn msvc_detours(app_path: String, command_line: String, workding_directory: String) -> (bool, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    println!("msvc detours");
    unsafe {
        //let lpApplicationName = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe";
        let lpApplicationName =  app_path.as_str();
        
        let lpCommandLine = command_line.as_str();

        let bInheritHandles = 0;   
        let dwCreationFlags = winapi::um::winbase::CREATE_DEFAULT_ERROR_MODE | winapi::um::winbase::CREATE_SUSPENDED;
        let lpEnvironment = std::ptr::null_mut();
        
        let appName = std::ffi::OsStr::new(lpApplicationName);
        let appNameWideChars: Vec<u16> = appName.encode_wide().chain(std::iter::once(0)).collect();

        let commandLine = std::ffi::OsStr::new(lpCommandLine);
        let mut commandLineWideChars: Vec<u16> = commandLine.encode_wide().chain(std::iter::once(0)).collect();
        
        let lpCurrentDirectory = std::ffi::OsStr::new(&workding_directory);
        let currentDirectoryWideChars: Vec<u16> = lpCurrentDirectory.encode_wide().chain(std::iter::once(0)).collect();
        
        
        let dllPath = tools::utils::get_working_path("redirect64.dll".to_string());
        if let Some(dllPath) = dllPath {

            let dllPath = std::ffi::CString::new(dllPath).unwrap();
            let dllPath = dllPath.as_ptr();

            let mut hStdOutputRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdOutputWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();

            let mut hStdErrorRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdErrorWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();

            let mut pipeAttributes = winapi::um::minwinbase::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: winapi::shared::minwindef::TRUE,
            };

            let ret = winapi::um::namedpipeapi::CreatePipe( &mut hStdOutputRead as winapi::shared::ntdef::PHANDLE, &mut hStdOutputWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret 
            {
                println!("create output pipe failed.")
            }

            let ret = winapi::um::namedpipeapi::CreatePipe(&mut hStdErrorRead as winapi::shared::ntdef::PHANDLE, &mut hStdErrorWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret 
            {
                println!("create error pipe failed.")
            }

            let lpProcessAttributes = std::ptr::null_mut();
            let lpThreadAttributes = std::ptr::null_mut();
            let mut lpStartupInfo: crate::detours::detours::_STARTUPINFOW = std::mem::MaybeUninit::zeroed().assume_init();
            lpStartupInfo.hStdError = hStdErrorWrite as *mut std::ffi::c_void;
            lpStartupInfo.hStdOutput = hStdOutputWrite as *mut std::ffi::c_void;
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

                let ret = winapi::um::processthreadsapi::ResumeThread(lpProcessInformation.hThread as _);
                if ret == winapi::shared::minwindef::TRUE as u32
                {
                    println!("ResumeThread success!");
                }
                else {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    println!("ResumeThread failed! error_code: {}.", error_code);
                }

                let mut chTmpStdOutputReadBuffer = vec![0; 128];
                let mut bytesStdOuputRead: winapi::shared::minwindef::DWORD = 0;
                let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();

                let bStdOutputRead = winapi::um::fileapi::ReadFile(
                    hStdOutputWrite, 
                    chTmpStdOutputReadBuffer.as_mut_ptr() as *mut _, 
                    chTmpStdOutputReadBuffer.len() as u32, 
                    &mut bytesStdOuputRead,
                    &mut overlapped
                );

                if bStdOutputRead == winapi::shared::minwindef::TRUE{
                    println!("read std output pipe size {:?}  {:?}", bytesStdOuputRead, chTmpStdOutputReadBuffer);
                }
                else {
                    println!("can't read std output pipe.");
                }

                winapi::um::handleapi::CloseHandle(hStdOutputWrite);
                winapi::um::handleapi::CloseHandle(hStdOutputRead);

                let mut chTmpStdErrorReadBuffer = vec![0; 128];
                let mut bytesStdErrorRead: winapi::shared::minwindef::DWORD = 0;
                let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();

                let bStdErrorRead = winapi::um::fileapi::ReadFile(
                    hStdErrorWrite, 
                    chTmpStdErrorReadBuffer.as_mut_ptr() as *mut _, 
                    chTmpStdErrorReadBuffer.len() as u32, 
                    &mut bytesStdErrorRead, 
                    &mut overlapped
                );

                if bStdErrorRead == winapi::shared::minwindef::TRUE {
                    println!("read std error pipe size {:?}  {:?}", bytesStdErrorRead, chTmpStdErrorReadBuffer);
                }   
                else {
                    println!("can't read std error pipe.");
                }


                winapi::um::handleapi::CloseHandle(hStdErrorWrite);
                winapi::um::handleapi::CloseHandle(hStdErrorRead);

                winapi::um::handleapi::CloseHandle(lpProcessInformation.hThread as _);
                winapi::um::handleapi::CloseHandle(lpProcessInformation.hProcess as _);

                println!("msvc detours end");

                return (true, std::sync::Arc::new(chTmpStdOutputReadBuffer), std::sync::Arc::new(chTmpStdErrorReadBuffer));
            }
            else {
                let error_code: u32 = winapi::um::errhandlingapi::GetLastError();
                println!("DetourCreateProcessWithDllExW failed! error_code: {}.", error_code);
                return (true, std::sync::Arc::new("".as_bytes().to_vec()), std::sync::Arc::new("".as_bytes().to_vec()));
            }
        }
        else
        {
            println!("can't fetch redirectdll. {:?}", dllPath);
            return (true, std::sync::Arc::new("".as_bytes().to_vec()), std::sync::Arc::new("".as_bytes().to_vec()));
        }
    }
} 