
use tokio::sync::mpsc::error;
use windows_sys::Win32 as win;

use crate::{communicate::unpackager::ReceivedCompileResources, detours::detours::DetourCreateProcessWithDllExW};
use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};

struct HandleBox {
    handle: win::Foundation::HANDLE,
}
 
impl HandleBox {
    pub fn new(h: win::Foundation::HANDLE) -> Self {
        Self { handle: h }
    }
 
    pub fn get(&self) -> &win::Foundation::HANDLE {
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

static VERSION_MAP_ENV: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, Vec<u16>>>> = std::sync::LazyLock::new(|| {
    let mut map = std::collections::HashMap::new();
    std::sync::Mutex::new(map)
});

pub unsafe fn pass_params_to_redirect(handle: win::Foundation::HANDLE, solution: &str, project: &str) {
    if !solution.is_empty() {
        let includes = crate::communicate::unpackager::RECEIVED_COMPILE_RESOURCES.files.get(project).map(|entry| {
            let includes = entry.value().iter().map(|item| item.key().to_string() + ":" + item.value()).collect::<Vec<String>>().join(";");
            includes
        }).unwrap_or_else(|| "".to_string());

        let arg = format!("solution:{}\r\nproject:{}\r\nreplica:{}\r\ndependency:{}", solution, project, tools::utils::access_replica_dir(), includes);
        let mut bytes: u32 = 0;
        let mut overlapped: win::System::IO::OVERLAPPED = std::mem::zeroed();
        let ret = win::Storage::FileSystem::WriteFile(
            handle,
            arg.as_bytes().as_ptr(),
            arg.len() as u32,
            &mut bytes,
            std::ptr::null_mut()
        );
    
        if ret == win::Foundation::FALSE || bytes == 0 {
            let error = win::Foundation::GetLastError();
            log::error!("write pipe error, failed code: {}", error);
        }
        else {
            //FlushFileBuffers(handle);
            log::trace!("send message by pipe {} {:?}", if bytes > 0 {"success."} else {"failed."}, arg);
        }
    }
    else {
        log::warn!("don't pass project name and project path, use current path and don't redirect.");
    }
    win::Foundation::CloseHandle(handle);
}

const SHARED_MEM_NAME: &str = "Local\\MspdbsrvRedirectSharedMem";
const SHARED_MEM_SIZE: usize = 256;

pub unsafe fn pass_params_to_mspdbsrv(solution: &str) {
    let name = SHARED_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();

    let h_map = win::System::Memory::CreateFileMappingW(
        win::Foundation::INVALID_HANDLE_VALUE,
        std::ptr::null_mut(),
        win::System::Memory::PAGE_READWRITE,
        0,
        SHARED_MEM_SIZE as u32,
        name.as_ptr()
    );

    if h_map.is_null() {
        let error = win::Foundation::GetLastError();
        log::error!("CreateFileMappingW failed! error code: {}.", error);
        return;
    }

    let buf = win::System::Memory::MapViewOfFile(
        h_map,
        win::System::Memory::FILE_MAP_ALL_ACCESS,
        0,
        0,
        SHARED_MEM_SIZE as usize
    );

    if buf.Value.is_null() {
        let error = win::Foundation::GetLastError();
        log::error!("MapViewOfFile failed! error code: {}.", error);
        win::Foundation::CloseHandle(h_map);
        return;
    }

    let data = format!("solution:{}\r\nreplica:{}\r\n", solution, tools::utils::access_replica_dir());
    let bytes = data.as_bytes();
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.Value as *mut u8, bytes.len());

    let _ = windows_sys::Win32::System::Memory::FlushViewOfFile(buf.Value as *const std::ffi::c_void, data.len() as usize);
    win::System::Memory::UnmapViewOfFile(buf);
}

fn replace_includes_path_by_replica(includes: &std::ffi::OsString) -> std::borrow::Cow<'_, std::ffi::OsString> {
    let replica_dir = tools::utils::access_replica_dir();

    if !replica_dir.is_empty() {
        let modified_includes: Vec<String> = includes.to_string_lossy().split(";")
        .filter(|include| !include.contains("Auxiliary"))
        .filter(|include| !include.is_empty())
        .filter(|include| include.contains(r":\"))
        .map(|include| {
            if let Some(index) = include.find(r"MSVC\") {
                format!(r#"{}\{}"#, replica_dir, &include[index..])
            }
            else if let Some(index) = include.find(r"Windows Kits\") {
                format!(r#"{}\{}"#, replica_dir,  &include[index..])
            }
            else {
                include.to_string()
            }
        }).collect();
        let mut joined = modified_includes.join(";");
        joined.push(';');
        return std::borrow::Cow::Owned(std::ffi::OsString::from(joined));
    }
    else {
        return std::borrow::Cow::Borrowed(includes);
    }
}

fn extract_version_from_path(app: &String) -> Option<String> {

    let p = std::path::Path::new(app);
    let comps: Vec<_> = p.components()
        .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
        .collect();

    for (i, comp) in comps.iter().enumerate() {
        if comp.eq_ignore_ascii_case("MSVC") {
            if let Some(ver) = comps.get(i + 1) {
                if ver.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
                    return Some(ver.clone());
                }
            }
        }
    }
    None
}

pub fn msvc_detours(solution: String, project: String, app: String, command: String, workding_dir: String,
    envs: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>, out_err_stream: &crate::compiler::msvc::OutAndErrStream) -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {

    unsafe {
        let lpApplicationName = app.as_str();

        let lpCommandLine = command.as_str();

        let bInheritHandles = win::Foundation::TRUE;
        let dwCreationFlags = win::System::Threading::CREATE_DEFAULT_ERROR_MODE | win::System::Threading::CREATE_SUSPENDED | win::System::Threading::CREATE_UNICODE_ENVIRONMENT;

        let mut env_block: Vec<u16> = Vec::new();
        let version = extract_version_from_path(&app);
        if let Some(ver) = &version {
            if let Some(val) = VERSION_MAP_ENV.lock().unwrap().get(ver) {
                env_block = val.clone();
            }
        }

        if env_block.is_empty() {
            std::env::var("SystemRoot").map(|value| {
                let mut env = std::ffi::OsString::from("SystemRoot");
                env.push("=");
                env.push(value);
                env_block.extend(env.encode_wide());
                env_block.push(0);
            });
            
            if let Ok(value) = std::env::var("TMP") {
                let mut env = std::ffi::OsString::from("TMP");
                env.push("=");
                env.push(value);
                env_block.extend(env.encode_wide());
                env_block.push(0);
            }
            else {
                std::env::var("TEMP").map(|value| {
                    let mut env = std::ffi::OsString::from("TEMP");
                    env.push("=");
                    env.push(value);
                    env_block.extend(env.encode_wide());
                    env_block.push(0);
                });
            }
            // must set SystemRoot TMP/TEMP envs, must not set VS_UNICODE_OUTPUT envs. i don't know why, it is test result.
            if !envs.is_empty() {
                for (key, value) in envs.iter() {
                    if  key.to_string_lossy().starts_with("VS_UNICODE_OUTPUT") {
                        continue;
                    }
    
                    if key.to_string_lossy().to_lowercase() == "include" {
                        let replaced = replace_includes_path_by_replica(&value);
                        let mut pair = key.clone();
                        pair.push("=");
                        pair.push(replaced.into_owned());
                        env_block.extend(pair.encode_wide());
                    }
                    else if key.to_string_lossy().to_lowercase() == "external_include" {
                        let replaced = replace_includes_path_by_replica(&value);
                        let mut pair = key.clone();
                        pair.push("=");
                        pair.push(replaced.into_owned());
                        env_block.extend(pair.encode_wide());
                    }
                    else {
                        let mut pair = key.clone();
                        pair.push("=");
                        pair.push(value);
                        env_block.extend(pair.encode_wide());
                    }
    
                    env_block.push(0);
                }
                env_block.push(0);
            }
            else {
                env_block.push(0);
            }

            VERSION_MAP_ENV.lock().unwrap().insert(version.unwrap(), env_block.clone());
        }
        else {
            log::debug!("use cached env block for version {}.", app);
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

            let mut pipeAttributes = win::Security::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<win::Security::SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: win::Foundation::TRUE,
            };

            let mut hStdInRead: win::Foundation::HANDLE = std::ptr::null_mut();
            let mut hStdInWrite: win::Foundation::HANDLE = std::ptr::null_mut();
            let ret = win::System::Pipes::CreatePipe(&mut hStdInRead, &mut hStdInWrite, &mut pipeAttributes, 0);
            if win::Foundation::FALSE == ret {
                log::error!("create input pipe failed.")
            }
            else {
                win::Foundation::SetHandleInformation(hStdInWrite, win::Foundation::HANDLE_FLAG_INHERIT, 0);
            }
            
            let mut hStdOutputRead: win::Foundation::HANDLE = std::ptr::null_mut();
            let mut hStdOutputWrite: win::Foundation::HANDLE = std::ptr::null_mut();
            let ret = win::System::Pipes::CreatePipe( &mut hStdOutputRead, &mut hStdOutputWrite, &mut pipeAttributes, 0);
            if win::Foundation::FALSE == ret {
                log::error!("create output pipe failed.")
            }
            else {
                win::Foundation::SetHandleInformation(hStdOutputRead, win::Foundation::HANDLE_FLAG_INHERIT, 0);
            }

            let mut hStdErrorRead: win::Foundation::HANDLE = std::ptr::null_mut();
            let mut hStdErrorWrite: win::Foundation::HANDLE = std::ptr::null_mut();
            let ret = win::System::Pipes::CreatePipe(&mut hStdErrorRead, &mut hStdErrorWrite, &mut pipeAttributes, 0);
            if win::Foundation::FALSE == ret {
                log::error!("create error pipe failed.")
            }
            else {
                win::Foundation::SetHandleInformation(hStdErrorRead, win::Foundation::HANDLE_FLAG_INHERIT, 0);
            }

            let lpProcessAttributes = std::ptr::null_mut();
            let lpThreadAttributes = std::ptr::null_mut();
            let mut lpStartupInfo: crate::detours::detours::_STARTUPINFOW = std::mem::MaybeUninit::zeroed().assume_init();
            lpStartupInfo.cb = std::mem::size_of::<crate::detours::detours::_STARTUPINFOW>() as u32;
            lpStartupInfo.hStdInput = hStdInRead as *mut std::ffi::c_void;            
            lpStartupInfo.hStdOutput = hStdOutputWrite as *mut std::ffi::c_void;
            lpStartupInfo.hStdError = hStdErrorWrite as *mut std::ffi::c_void;
            lpStartupInfo.dwFlags |= win::System::Threading::STARTF_USESTDHANDLES;

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
        
            win::Foundation::CloseHandle(hStdOutputWrite);
            win::Foundation::CloseHandle(hStdErrorWrite);
            win::Foundation::CloseHandle(hStdInRead);

            if ret == win::Foundation::TRUE {
                let stdoutstream = std::sync::Arc::new(out_err_stream.stdout.to_owned());
                let stderrstream = std::sync::Arc::new(out_err_stream.stderr.to_owned());

                pass_params_to_redirect(hStdInWrite, &solution, &project);
                pass_params_to_mspdbsrv(&solution);

                let ret = win::System::Threading::ResumeThread(lpProcessInformation.hThread as _);
                if ret == win::Foundation::FALSE as u32 {
                    let error_code = win::Foundation::GetLastError();
                    log::error!("ResumeThread failed! error code: {}.", error_code);
                }

                let hProcessBox = HandleBox::new(lpProcessInformation.hProcess as win::Foundation::HANDLE);
                let hProcessForStdout = hProcessBox.clone();
                let hProcessForStderr = hProcessBox.clone();

                let hStdOutputReadBox = HandleBox::new(hStdOutputRead);
                
                let task_stdout = std::thread::Builder::new().name("build-stdout-reader".into()).spawn(move || {

                    let mut chTmpStdOutputReadBuffer = [0u8; 1024];
                    let mut bytesStdOuputRead: u32 = 0;
                    
                    let mut line = Vec::new();
                    let mut stdout = Vec::new();
                    loop {
                        let mut avail: u32 = 0;
                        let peek = win::System::Pipes::PeekNamedPipe(
                            *hStdOutputReadBox.get(),
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            &mut avail,
                            std::ptr::null_mut()
                        );

                        if peek == win::Foundation::FALSE {
                            break;
                        }

                        if avail == 0 {
                            let wait = win::System::Threading::WaitForSingleObject(
                                *hProcessForStdout.get(), 0
                            );
                            if wait == 0 {  // WAIT_OBJECT_0: cl.exe has exited
                                log::debug!("stdout reader: cl.exe exited and no more data, exiting");
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            continue;
                        }

                        chTmpStdOutputReadBuffer.fill(0);
                        let bStdOutputRead = win::Storage::FileSystem::ReadFile(
                            *hStdOutputReadBox.get(),
                            chTmpStdOutputReadBuffer.as_mut_ptr() as *mut _, 
                            chTmpStdOutputReadBuffer.len() as u32,
                            &mut bytesStdOuputRead,
                            std::ptr::null_mut()
                        );
                        
                        if bStdOutputRead == win::Foundation::FALSE || bytesStdOuputRead == 0 {
                            let error = win::Foundation::GetLastError();
                            if error != win::Foundation::ERROR_BROKEN_PIPE && error != win::Foundation::ERROR_INVALID_HANDLE {
                                log::error!("can't read stdout pipe. error code: {}", error);
                            }
                            break;
                        }

                        if bytesStdOuputRead > 2 && chTmpStdOutputReadBuffer[(bytesStdOuputRead - 1) as usize] == b'\n' && chTmpStdOutputReadBuffer[(bytesStdOuputRead - 2) as usize] == b'\r' {
                            if line.is_empty() {
                                stdoutstream.try_send(chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize].to_vec()).unwrap_or_else(|_| {
                                    log::error!("try send stdout stream failed.");
                                });
                            }
                            else {
                                line.extend_from_slice(&chTmpStdOutputReadBuffer[..bytesStdOuputRead as usize]);
                                stdoutstream.try_send(line[..].to_vec()).unwrap_or_else(|_| {
                                    log::error!("try send stdout stream failed.");
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
                }).unwrap();

                let hStdErrorReadBox = HandleBox::new(hStdErrorRead);
                let task_stderr = std::thread::Builder::new().name("build-stderr-reader".into()).spawn(move || {
                    let mut chTmpStdErrorReadBuffer =  [0u8; 1024];
                    let mut bytesStdErrorRead: u32 = 0;

                    let mut stderr = Vec::new();

                    loop {

                        let mut avail: u32 = 0;
                        let peek = win::System::Pipes::PeekNamedPipe(
                            *hStdErrorReadBox.get(),
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            &mut avail,
                            std::ptr::null_mut()
                        );

                        if peek == win::Foundation::FALSE {
                            break;
                        }

                        if avail == 0 {
                            let wait = win::System::Threading::WaitForSingleObject(
                                *hProcessForStderr.get(), 0
                            );
                            if wait == 0 {  // WAIT_OBJECT_0: cl.exe has exited
                                log::debug!("stderr reader: cl.exe exited and no more data, exiting");
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            continue;
                        }

                        chTmpStdErrorReadBuffer.fill(0);
                        let bStdErrorRead = win::Storage::FileSystem::ReadFile(
                            *hStdErrorReadBox.get(), 
                            chTmpStdErrorReadBuffer.as_mut_ptr(), 
                            chTmpStdErrorReadBuffer.len() as u32, 
                            &mut bytesStdErrorRead, 
                            std::ptr::null_mut()
                        );
                        
                        if bStdErrorRead == win::Foundation::FALSE || bytesStdErrorRead == 0 {
                            let error = win::Foundation::GetLastError();
                            if error != win::Foundation::ERROR_BROKEN_PIPE && error != win::Foundation::ERROR_INVALID_HANDLE {
                                log::error!("can't read stderr pipe. error code: {}", error);
                            }
                            break;
                        }
                        
                        let stderrstream_ = stderrstream.clone();
                        stderrstream_.try_send(chTmpStdErrorReadBuffer[..bytesStdErrorRead as usize].to_vec()).unwrap_or_else(|_| {
                            log::error!("try send stderr stream failed.");
                        });

                        stderr.extend_from_slice(&chTmpStdErrorReadBuffer[..bytesStdErrorRead as usize]);
                    }
                    drop(stderrstream);
                    return stderr;
                }).unwrap();
                
                let stdout = task_stdout.join().unwrap();
                let stderr = task_stderr.join().unwrap();
                
                let mut code: u32 = 0;
                win::System::Threading::GetExitCodeProcess(*hProcessBox.get(), &mut code as _);
                log::info!("msvc detours: {} end with exit code: {:#x} pid: {:?}", project, code, lpProcessInformation.dwProcessId);

                win::Foundation::CloseHandle(lpProcessInformation.hThread as _);
                win::Foundation::CloseHandle(*hProcessBox.get());

                if !hStdOutputRead.is_null() {
                    win::Foundation::CloseHandle(hStdOutputRead);
                }

                if !hStdErrorRead.is_null() {
                    win::Foundation::CloseHandle(hStdErrorRead);
                }

                return (code, std::sync::Arc::new(stdout), std::sync::Arc::new(stderr));
            }
            else {
                if !hStdOutputRead.is_null() {
                    win::Foundation::CloseHandle(hStdOutputRead);
                }
                if !hStdErrorRead.is_null() {
                    win::Foundation::CloseHandle(hStdErrorRead);
                }

                let code = win::Foundation::GetLastError();
                let error = format!("DetourCreateProcessWithDllExW failed! error code: {}. error message: {}.", code, tools::utils::get_winapi_error_message(code));
                log::error!("{}", &error);
                return (code, std::sync::Arc::new(Vec::new()), std::sync::Arc::new(error.bytes().collect()));
            }
        }
        else
        {
            log::error!("access redirect64.dll failed. {:?}", redirect_dll_path);
            return (1, std::sync::Arc::new(Vec::new()), std::sync::Arc::new(Vec::new()));
        }
    }
}


