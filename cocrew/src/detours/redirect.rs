
use tokio::sync::mpsc::error;
use winapi::{shared::minwindef::LPDWORD, um::{errhandlingapi::GetLastError, handleapi::CloseHandle, processthreadsapi::CreateProcessW}};

use crate::detours::detours::DetourCreateProcessWithDllExW;
use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};

struct HandleBox {
    handle: winapi::shared::ntdef::HANDLE,
}
 
impl HandleBox {
    pub fn new(h: winapi::shared::ntdef::HANDLE) -> Self {
        Self { handle: h }
    }
 
    pub fn get(&self) -> &winapi::shared::ntdef::HANDLE {
        &self.handle
    }

    fn clone(&self) -> Self {
        Self { handle: self.handle.clone() }
    }
}
 
unsafe impl Send for HandleBox {}
unsafe impl Sync for HandleBox {}

pub unsafe fn pass_object_name_to_redriect(handle: winapi::shared::ntdef::HANDLE, project: &str) {
    if !project.is_empty() {
        let arg = format!("project:{}\nreplica:{}\n", project, tools::utils::access_replica_dir());
        let mut bytes: winapi::shared::minwindef::DWORD = 0;
        let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
        let ret = winapi::um::fileapi::WriteFile(
            handle,
            arg.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
            arg.len() as u32,
            &mut bytes,
            &mut overlapped
        );
    
        if ret == winapi::shared::minwindef::FALSE || bytes == 0 {
            let error = winapi::um::errhandlingapi::GetLastError();
            log::error!("write pipe error, failed code: {}", error);
        }
        else {
            //winapi::um::fileapi::FlushFileBuffers(handle);
            log::trace!("send message by pipe {} {:?}", if bytes > 0 {"success."} else {"failed."}, arg);
        }
    }
    else {
        log::warn!("don't pass project name and project path, use current path and don't redirect.");
    }
    CloseHandle(handle);
}

pub fn msvc_detours(project: String, app_path: String, command: String, workding_directory: String) -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    //TODO workding_directory should be also use redirect.
     
    unsafe {
        //let lpApplicationName = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe";
        let lpApplicationName =  app_path.as_str();
        
        let lpCommandLine = command.as_str();

        let bInheritHandles = winapi::shared::minwindef::TRUE;
        let dwCreationFlags = winapi::um::winbase::CREATE_DEFAULT_ERROR_MODE | winapi::um::winbase::CREATE_SUSPENDED;
        let lpEnvironment = std::ptr::null_mut();
        
        let appName = std::ffi::OsStr::new(lpApplicationName);
        let appNameWideChars: Vec<u16> = appName.encode_wide().chain(std::iter::once(0)).collect();

        let commandLine = std::ffi::OsStr::new(lpCommandLine);
        let mut commandLineWideChars: Vec<u16> = commandLine.encode_wide().chain(std::iter::once(0)).collect();
        
        let lpCurrentDirectory = std::ffi::OsStr::new(&workding_directory);
        let currentDirectoryWideChars: Vec<u16> = lpCurrentDirectory.encode_wide().chain(std::iter::once(0)).collect();
        
        //TODO performance issue when build time frequently call this function.
        let dllPath = tools::utils::access_working_path("redirect64.dll");
        //log::debug!("redirect64.dll {:?}", dllPath);
        if let Some(dllPath) = dllPath {

            let dllPath = std::ffi::CString::new(dllPath).unwrap();
            let dllPath = dllPath.as_ptr();

            let mut pipeAttributes = winapi::um::minwinbase::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: winapi::shared::minwindef::TRUE,
            };

            let mut hStdInRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdInWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let ret = winapi::um::namedpipeapi::CreatePipe(&mut hStdInRead as winapi::shared::ntdef::PHANDLE, &mut hStdInWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret {
                log::error!("create input pipe failed.")
            }
            
            let mut hStdOutputRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdOutputWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let ret = winapi::um::namedpipeapi::CreatePipe( &mut hStdOutputRead as winapi::shared::ntdef::PHANDLE, &mut hStdOutputWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret {
                log::error!("create output pipe failed.")
            }

            let mut hStdErrorRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdErrorWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let ret = winapi::um::namedpipeapi::CreatePipe(&mut hStdErrorRead as winapi::shared::ntdef::PHANDLE, &mut hStdErrorWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret {
                log::error!("create error pipe failed.")
            }

            let lpProcessAttributes = std::ptr::null_mut();
            let lpThreadAttributes = std::ptr::null_mut();
            let mut lpStartupInfo: crate::detours::detours::_STARTUPINFOW = std::mem::MaybeUninit::zeroed().assume_init();
            lpStartupInfo.hStdInput = hStdInRead as *mut std::ffi::c_void;            
            lpStartupInfo.hStdOutput = hStdOutputWrite as *mut std::ffi::c_void;
            lpStartupInfo.hStdError = hStdErrorWrite as *mut std::ffi::c_void;
            lpStartupInfo.dwFlags |=  winapi::um::winbase::STARTF_USESTDHANDLES;

            let mut lpProcessInformation: crate::detours::detours::_PROCESS_INFORMATION = std::mem::MaybeUninit::zeroed().assume_init();
            
            let ret = DetourCreateProcessWithDllExW(appNameWideChars.as_ptr(),
                commandLineWideChars.as_mut_ptr(), 
                lpProcessAttributes,
                lpThreadAttributes, 
                bInheritHandles, 
                dwCreationFlags, 
                lpEnvironment, 
                currentDirectoryWideChars.as_ptr(), 
                &mut lpStartupInfo as *mut _, 
                &mut lpProcessInformation as *mut _, 
                dllPath, 
                Option::None);
            
            CloseHandle(hStdOutputWrite);
            CloseHandle(hStdErrorWrite);

            if ret == winapi::shared::minwindef::TRUE {
                
                pass_object_name_to_redriect(hStdInWrite, &project);
                
                let ret = winapi::um::processthreadsapi::ResumeThread(lpProcessInformation.hThread as _);
                if ret == winapi::shared::minwindef::FALSE as u32 {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    log::error!("ResumeThread failed! error code: {}.", error_code);
                }

                let hStdOutputReadBox = HandleBox::new(hStdOutputRead);
                let task = std::thread::spawn(move || {

                    let mut chTmpStdOutputReadBuffer = vec![0; 512];
                    let mut bytesStdOuputRead: winapi::shared::minwindef::DWORD = 0;
                    let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
    
                    let mut stdout = Vec::new();
                    loop {
                        let bStdOutputRead = winapi::um::fileapi::ReadFile(
                            hStdOutputReadBox.get().to_owned(),
                            chTmpStdOutputReadBuffer.as_mut_ptr() as *mut _, 
                            chTmpStdOutputReadBuffer.len() as u32, 
                            &mut bytesStdOuputRead,
                            &mut overlapped
                        );
                        
                        if bStdOutputRead == winapi::shared::minwindef::FALSE || bytesStdOuputRead == 0 {
                            let error = winapi::um::errhandlingapi::GetLastError();
                            if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                
                            }
                            else {
                                log::error!("can't read stdout pipe. error code: {}", winapi::um::errhandlingapi::GetLastError());
                            }
                            break;
                        }
    
                        stdout.extend_from_slice(&chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize]);
                    }
                    return stdout;
                });
                
                let mut chTmpStdErrorReadBuffer = vec![0; 512];
                let mut bytesStdErrorRead: winapi::shared::minwindef::DWORD = 0;
                let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();

                let mut stderr = Vec::new();

                loop {
                    let bStdErrorRead = winapi::um::fileapi::ReadFile(
                        hStdErrorRead, 
                        chTmpStdErrorReadBuffer.as_mut_ptr() as *mut _, 
                        chTmpStdErrorReadBuffer.len() as u32, 
                        &mut bytesStdErrorRead, 
                        &mut overlapped
                    );
                    
                    if bStdErrorRead == winapi::shared::minwindef::FALSE || bytesStdErrorRead == 0 {
                        let error = winapi::um::errhandlingapi::GetLastError();
                        if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {

                        }
                        else {
                            log::error!("can't read stderr pipe. error code: {}", winapi::um::errhandlingapi::GetLastError());
                        }
                        break;
                    }

                    stderr.extend_from_slice(&chTmpStdErrorReadBuffer[..bytesStdErrorRead as usize]);

                }
                
                winapi::um::synchapi::WaitForSingleObject(lpProcessInformation.hProcess as winapi::um::winnt::HANDLE, winapi::um::winbase::INFINITE);
                let stdout = task.join().unwrap();

                let mut code: winapi::shared::minwindef::DWORD = 0;
                winapi::um::processthreadsapi::GetExitCodeProcess(lpProcessInformation.hProcess as winapi::um::winnt::HANDLE, &mut code as *mut winapi::shared::minwindef::DWORD);

                winapi::um::handleapi::CloseHandle(lpProcessInformation.hThread as _);
                winapi::um::handleapi::CloseHandle(lpProcessInformation.hProcess as _);

                log::info!("msvc detours end with exit code {}", code);

                return (code, std::sync::Arc::new(stdout), std::sync::Arc::new(stderr));
            }
            else {
                let code = winapi::um::errhandlingapi::GetLastError();
                log::error!("DetourCreateProcessWithDllExW failed! error code: {}. error message: {}.", code, tools::utils::get_winapi_error_message(code));
                return (code, std::sync::Arc::new(Vec::new()), std::sync::Arc::new(Vec::new()));
            }

            winapi::um::handleapi::CloseHandle(hStdOutputRead);
            winapi::um::handleapi::CloseHandle(hStdErrorRead);
            winapi::um::handleapi::CloseHandle(hStdInRead);
        }
        else
        {
            log::error!("can't fetch redirectdll. {:?}", dllPath);
            return (1, std::sync::Arc::new(Vec::new()), std::sync::Arc::new(Vec::new()));
        }
    }
}


