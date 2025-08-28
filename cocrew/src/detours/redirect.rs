
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

static REDIRECT_DLL_PATH: std::sync::LazyLock<Option<std::ffi::CString>> = std::sync::LazyLock::new(|| {
        if let Some(path) = tools::utils::access_working_path("redirect64.dll") {
            log::debug!("access redirect64 path successful. {:?}", path);
            return Some(std::ffi::CString::new(path).unwrap());
        }
        else {
            log::error!("access redirect64 path failed.");
            return None;
        }
    });

pub unsafe fn pass_params_to_redirect(handle: winapi::shared::ntdef::HANDLE, solution: &str, project: &str) {
    if !solution.is_empty() {
        let arg = format!("solution:{}\r\nproject:{}\r\nreplica:{}\r\n", solution, project, tools::utils::access_replica_dir());
        let mut bytes: winapi::shared::minwindef::DWORD = 0;
        let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
        let ret = winapi::um::fileapi::WriteFile(
            handle,
            arg.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
            arg.len() as u32,
            &mut bytes,
            std::ptr::null_mut()
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

pub fn msvc_detours(solution: String, project: String, app_path: String, command: String, workding_dir: String,
    envs: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>, out_err_stream: &crate::compiler::msvc::OutAndErrStream) -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {

    unsafe {
        let lpApplicationName = app_path.as_str();

        let lpCommandLine = command.as_str();

        let bInheritHandles = winapi::shared::minwindef::TRUE;
        let dwCreationFlags = winapi::um::winbase::CREATE_DEFAULT_ERROR_MODE | winapi::um::winbase::CREATE_SUSPENDED | winapi::um::winbase::CREATE_UNICODE_ENVIRONMENT;

        let mut env_block: Vec<u16> = Vec::new();

        let mut env = std::ffi::OsString::from("SystemRoot");
        env.push("=");
        env.push(r"C:\WINDOWS");
        env_block.extend(env.encode_wide());
        env_block.push(0);

        if !envs.is_empty() {
            for (key, value) in envs.iter() {
                let mut pair = key.clone();
                pair.push("=");
                pair.push(value);
                
                env_block.extend(pair.encode_wide());
                env_block.push(0);
            }
            env_block.push(0);
        }
        else {
            let mut env = std::ffi::OsString::from("INCLUDE");
            env.push("=");
            env.push(r"");
            env_block.extend(env.encode_wide());
            env_block.push(0);

            env_block.push(0);
        }

        let lpEnvironment = env_block.as_mut_ptr() as *mut std::ffi::c_void;
        
        let appName = std::ffi::OsStr::new(lpApplicationName);
        let appNameWideChars: Vec<u16> = appName.encode_wide().chain(std::iter::once(0)).collect();

        let commandLine = std::ffi::OsStr::new(lpCommandLine);
        let mut commandLineWideChars: Vec<u16> = commandLine.encode_wide().chain(std::iter::once(0)).collect();
        
        let lpCurrentDirectory = std::ffi::OsStr::new(&workding_dir);
        let currentDirectoryWideChars: Vec<u16> = lpCurrentDirectory.encode_wide().chain(std::iter::once(0)).collect();

        let redirect_dll_path = &*REDIRECT_DLL_PATH;
        if let Some(dllPath) = redirect_dll_path {
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
            else {
                winapi::um::handleapi::SetHandleInformation(hStdInWrite, winapi::um::winbase::HANDLE_FLAG_INHERIT, 0);
            }
            
            let mut hStdOutputRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdOutputWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let ret = winapi::um::namedpipeapi::CreatePipe( &mut hStdOutputRead as winapi::shared::ntdef::PHANDLE, &mut hStdOutputWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret {
                log::error!("create output pipe failed.")
            }
            else {
                winapi::um::handleapi::SetHandleInformation(hStdOutputRead, winapi::um::winbase::HANDLE_FLAG_INHERIT, 0);
            }

            let mut hStdErrorRead: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut hStdErrorWrite: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let ret = winapi::um::namedpipeapi::CreatePipe(&mut hStdErrorRead as winapi::shared::ntdef::PHANDLE, &mut hStdErrorWrite as winapi::shared::ntdef::PHANDLE, &mut pipeAttributes, 0);
            if winapi::shared::minwindef::FALSE == ret {
                log::error!("create error pipe failed.")
            }
            else {
                winapi::um::handleapi::SetHandleInformation(hStdErrorRead, winapi::um::winbase::HANDLE_FLAG_INHERIT, 0);
            }

            let lpProcessAttributes = std::ptr::null_mut();
            let lpThreadAttributes = std::ptr::null_mut();
            let mut lpStartupInfo: crate::detours::detours::_STARTUPINFOW = std::mem::MaybeUninit::zeroed().assume_init();
            lpStartupInfo.cb = std::mem::size_of::<crate::detours::detours::_STARTUPINFOW>() as u32;
            lpStartupInfo.hStdInput = hStdInRead as *mut std::ffi::c_void;            
            lpStartupInfo.hStdOutput = hStdOutputWrite as *mut std::ffi::c_void;
            lpStartupInfo.hStdError = hStdErrorWrite as *mut std::ffi::c_void;
            lpStartupInfo.dwFlags |=  winapi::um::winbase::STARTF_USESTDHANDLES;

            let mut lpProcessInformation: crate::detours::detours::_PROCESS_INFORMATION = std::mem::MaybeUninit::zeroed().assume_init();
               
            let ret = DetourCreateProcessWithDllExW(
                appNameWideChars.as_ptr(),
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
                Option::None
            );
        
            CloseHandle(hStdOutputWrite);
            CloseHandle(hStdErrorWrite);
            CloseHandle(hStdInRead);

            if ret == winapi::shared::minwindef::TRUE {
                let stdoutstream = out_err_stream.stdout.to_owned();
                let stderrstream = out_err_stream.stderr.to_owned();

                pass_params_to_redirect(hStdInWrite, &solution, &project);

                let ret = winapi::um::processthreadsapi::ResumeThread(lpProcessInformation.hThread as _);
                if ret == winapi::shared::minwindef::FALSE as u32 {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    log::error!("ResumeThread failed! error code: {}.", error_code);
                }

                let hStdOutputReadBox = HandleBox::new(hStdOutputRead);
                let task = std::thread::spawn(move || {

                    let mut chTmpStdOutputReadBuffer = [0u8; 1024];
                    let mut bytesStdOuputRead: winapi::shared::minwindef::DWORD = 0;
                    
                    let mut line = Vec::new();
                    let mut stdout = Vec::new();
                    loop {
                        chTmpStdOutputReadBuffer.fill(0);
                        let bStdOutputRead = winapi::um::fileapi::ReadFile(
                            *hStdOutputReadBox.get(),
                            chTmpStdOutputReadBuffer.as_mut_ptr() as *mut _, 
                            chTmpStdOutputReadBuffer.len() as u32,
                            &mut bytesStdOuputRead,
                            std::ptr::null_mut()
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

                        if bytesStdOuputRead > 2 && chTmpStdOutputReadBuffer[(bytesStdOuputRead - 1) as usize] == b'\n' && chTmpStdOutputReadBuffer[(bytesStdOuputRead - 2) as usize] == b'\r' {
                            if line.is_empty() {
                                stdoutstream.send(chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize].to_vec()).unwrap_or_else(|_| {
                                    log::error!("send stdout stream failed.");
                                });
                            }
                            else {
                                line.extend_from_slice(&chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize]);
                                stdoutstream.send(line[..].to_vec()).unwrap_or_else(|_| {
                                    log::error!("send stdout stream failed.");
                                });
                                line.clear();
                            }
                        }
                        else {
                            line.extend_from_slice(&chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize]);
                        }

                        stdout.extend_from_slice(&chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize]);
                    }
                    drop(stdoutstream);
                    return stdout;
                });
                
                let mut chTmpStdErrorReadBuffer =  [0u8; 1024];
                let mut bytesStdErrorRead: winapi::shared::minwindef::DWORD = 0;

                let mut stderr = Vec::new();

                loop {
                    chTmpStdErrorReadBuffer.fill(0);
                    let bStdErrorRead = winapi::um::fileapi::ReadFile(
                        hStdErrorRead, 
                        chTmpStdErrorReadBuffer.as_mut_ptr() as *mut _, 
                        chTmpStdErrorReadBuffer.len() as u32, 
                        &mut bytesStdErrorRead, 
                        std::ptr::null_mut()
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

                    stderrstream.send(chTmpStdErrorReadBuffer[..bytesStdErrorRead as usize].to_vec()).unwrap_or_else(|_| {
                        log::error!("send stderr stream failed.");
                    });

                    stderr.extend_from_slice(&chTmpStdErrorReadBuffer[..bytesStdErrorRead as usize]);
                }
                drop(stderrstream);
                
                winapi::um::synchapi::WaitForSingleObject(lpProcessInformation.hProcess as winapi::um::winnt::HANDLE, winapi::um::winbase::INFINITE);
                
                let stdout = task.join().unwrap();

                let mut code: winapi::shared::minwindef::DWORD = 0;
                winapi::um::processthreadsapi::GetExitCodeProcess(lpProcessInformation.hProcess as winapi::um::winnt::HANDLE, &mut code as *mut winapi::shared::minwindef::DWORD);

                winapi::um::handleapi::CloseHandle(lpProcessInformation.hThread as _);
                winapi::um::handleapi::CloseHandle(lpProcessInformation.hProcess as _);

                log::info!("msvc detours {} end with exit code {}. error message: {}.", project, code, tools::utils::get_winapi_error_message(code));
                if !hStdOutputRead.is_null() {
                    winapi::um::handleapi::CloseHandle(hStdOutputRead);
                }

                if !hStdErrorRead.is_null() {
                    winapi::um::handleapi::CloseHandle(hStdErrorRead);
                }

                return (code, std::sync::Arc::new(stdout), std::sync::Arc::new(stderr));
            }
            else {
                if !hStdOutputRead.is_null() {
                    winapi::um::handleapi::CloseHandle(hStdOutputRead);
                }
                if !hStdErrorRead.is_null() {
                    winapi::um::handleapi::CloseHandle(hStdErrorRead);
                }

                let code = winapi::um::errhandlingapi::GetLastError();
                log::error!("DetourCreateProcessWithDllExW failed! error code: {}. error message: {}.", code, tools::utils::get_winapi_error_message(code));
                return (code, std::sync::Arc::new(Vec::new()), std::sync::Arc::new(Vec::new()));
            }
        }
        else
        {
            log::error!("access redirect64.dll failed. {:?}", redirect_dll_path);
            return (1, std::sync::Arc::new(Vec::new()), std::sync::Arc::new(Vec::new()));
        }
    }
}


