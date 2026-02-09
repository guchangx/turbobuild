use windows_sys::Win32 as win;

use crate::log;

async fn connect() -> std::result::Result<tokio::net::windows::named_pipe::NamedPipeClient, std::io::Error> {
    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";
    let client = loop {
        match tokio::net::windows::named_pipe::ClientOptions::new()
            .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
            .open(PIPE_NAME) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_PIPE_BUSY as i32) => (),
            Err(e) => {
                crate::log!(error, "failed to connect to named pipe: {}", e);
                return Err(e)
            },
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    };
    crate::log!(info,"connected to named pipe: {}", PIPE_NAME);
    return Ok(client);
}

pub struct MirrorSysCall {
    pub cid: u32,
    pub api: String,
    pub args: std::collections::HashMap<String, String>,
    pub responder: tokio::sync::oneshot::Sender<std::collections::HashMap<String, String>>,
}

pub struct Channel {
    pub tx: tokio::sync::mpsc::Sender<MirrorSysCall>,
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<MirrorSysCall>>>,
}

pub static REDIRECT_SYS_CALL_CHANNEL: std::sync::LazyLock<Channel> = std::sync::LazyLock::new(|| {
    let (tx, rx) = tokio::sync::mpsc::channel::<MirrorSysCall>(512);
    let channel = Channel { tx, rx: std::sync::Mutex::new(Some(rx)) };
    return channel;
});

struct EventGuard(win::Foundation::HANDLE);
impl Drop for EventGuard {
    fn drop(&mut self) { unsafe { win::Foundation::CloseHandle(self.0); } }
}

unsafe fn redirect_syscall_2_cocrew() {

    use std::os::windows::ffi::OsStrExt;

    let runtime = crate::REDIRECT_RUNTIME.handle().clone();
    let runtime_ = runtime.clone();
    let _ = runtime.spawn_blocking(move || {

        let name = std::ffi::OsString::from(r"\\.\pipe\os_operate_request_pipe");
        let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
        
        type Map = std::collections::HashMap<u32, tokio::sync::oneshot::Sender<std::collections::HashMap<String, String>>>;
        let responders: std::sync::Arc<std::sync::Mutex<Map>> = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        
        let hex = "tb".as_bytes().iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let template_value = u64::from_str_radix(&hex, 16)
            .unwrap_or(0) as win::Foundation::HANDLE;

        for i in 0..6 {
            let mut pipe_handle = win::Storage::FileSystem::CreateFileW(name.as_ptr(), 
                win::Foundation::GENERIC_WRITE | win::Foundation::GENERIC_READ,
                0,
                std::ptr::null_mut(),  
                win::Storage::FileSystem::OPEN_EXISTING, 
                win::Storage::FileSystem::FILE_FLAG_OVERLAPPED,
                template_value as _
            );

            if pipe_handle.is_null() || pipe_handle == win::Foundation::INVALID_HANDLE_VALUE {
                let error = win::Foundation::GetLastError();
                crate::log!(info, "connecting to named pipe failed and wait: {} current process: {} error code: {} message: {}", r"\\.\pipe\os_operate_request_pipe", std::process::id(), error, tools::utils::get_winapi_error_message(error));
                if win::System::Pipes::WaitNamedPipeW(name.as_ptr(), 300 * i) == win::Foundation::TRUE {

                }
                else {
                    let error = win::Foundation::GetLastError();
                    crate::log!(info, "redirect_syscall_2_cocrew WaitNamedPipeW failed, current process: {} error code: {}, message: {}", std::process::id(), error, tools::utils::get_winapi_error_message(error));
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
            }

            let responders = responders.clone();
            for _ in 0..3 {

                if pipe_handle.is_null() || pipe_handle == win::Foundation::INVALID_HANDLE_VALUE {
                
                    pipe_handle = win::Storage::FileSystem::CreateFileW(name.as_ptr(), 
                        win::Foundation::GENERIC_WRITE | win::Foundation::GENERIC_READ,
                        0,
                        std::ptr::null_mut(),  
                        win::Storage::FileSystem::OPEN_EXISTING, 
                        win::Storage::FileSystem::FILE_FLAG_OVERLAPPED,  
                         std::ptr::null_mut()
                    );
                }

                if !pipe_handle.is_null() && pipe_handle != win::Foundation::INVALID_HANDLE_VALUE {
                    
                    let mut mode: u32 = win::System::Pipes::PIPE_READMODE_MESSAGE;
                    if win::System::Pipes::SetNamedPipeHandleState(pipe_handle,
                        &mut mode,
                        std::ptr::null_mut(),
                        std::ptr::null_mut()
                    ) == win::Foundation::FALSE {
                        let error = win::Foundation::GetLastError();
                        crate::log!(error, "SetNamedPipeHandleState failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                    }

                    let mut pid: u32 = 0;
                    win::System::Pipes::GetNamedPipeServerProcessId(pipe_handle as *mut _, &mut pid as *mut u32);
                    crate::log!(info, "connected to syscall namedpipe server process id: {} current process: {}", pid, std::process::id());
                    
                    let pipe_handle = tools::ptr::HandleBox::new(pipe_handle);
                    let pipe_handle_ = pipe_handle.clone();

                    let responders_ = responders.clone();
                    // receive command from named pipe and send it to sync caller in functions
                    let _handle_r = runtime_.spawn_blocking(move || {
                        let mut moredata = Vec::new();
                        loop {
                            let mut buffer: [u8; 512] = [0 as u8; 512];
                            let mut bytes: u32= 0;
                            
                            let event = win::System::Threading::CreateEventW(
                                std::ptr::null_mut(),
                                win::Foundation::TRUE, 
                                win::Foundation::FALSE,
                                std::ptr::null_mut()
                            );

                            let mut overlapped: win::System::IO::OVERLAPPED = std::mem::zeroed();
                            overlapped.hEvent = event;

                            let result = win::Storage::FileSystem::ReadFile(
                                pipe_handle.get().to_owned(),
                                buffer.as_mut_ptr() as *mut _,
                                buffer.len() as u32,
                                &mut bytes,
                                &mut overlapped,
                            );
                            
                            let _eg = EventGuard(event);

                            if result == win::Foundation::FALSE {

                                let err = win::Foundation::GetLastError();

                                if err == win::Foundation::ERROR_IO_PENDING || err == win::Foundation::ERROR_MORE_DATA {

                                    if err == win::Foundation::ERROR_IO_PENDING {
                                        win::System::Threading::WaitForSingleObject(event, win::System::Threading::INFINITE);
                                    }
                                    
                                    let mut final_bytes: u32 = 0;
                                    let result = win::System::IO::GetOverlappedResult(
                                        pipe_handle.get().to_owned(),
                                        &mut overlapped,
                                        &mut final_bytes,
                                        win::Foundation::FALSE
                                    );

                                    let mut is_moredata = false;
                                    if result == win::Foundation::FALSE {
                                        let err = win::Foundation::GetLastError();
                                        if err == win::Foundation::ERROR_MORE_DATA {
                                            is_moredata = true;
                                        }
                                        else
                                        {
                                            crate::log!(error, "GetOverlappedResult failed, error code: {}, message: {}", err, tools::utils::get_winapi_error_message(err));
                                        }
                                    }
                                    
                                    if final_bytes == 0 {
                                        crate::log!(error, "response virtual syscall named pipe closed. final bytes is 0.");
                                    }
                                    else if final_bytes == buffer.len() as u32 {
                                        if is_moredata {
                                            moredata.extend_from_slice(&buffer[..final_bytes as usize]);
                                        }
                                        else {
                                            moredata.extend_from_slice(&buffer[..final_bytes as usize]);

                                            if moredata.is_empty() {
                                                crate::log!(error, "readfile buffer is empty");
                                            }
                                            else {
                                                let output = String::from_utf8_lossy(&moredata);

                                                let (id, command, args) = parse_mirror_command(&output);
                                                crate::log!(debug, "receive pipe sysycall id: {} command: {}", id, command);
                                                let option = { responders.lock().unwrap().remove(&id) };
                                                if let Some(responder) = option {
                                                    if let Err(e) = responder.send(args) {
                                                        crate::log!(info, "failed to send virtual syscall: {:?}", e);
                                                    }
                                                }
                                                else {
                                                    crate::log!(warn, "no responder found for id: {} syscall: {} responders: {:?}", id, command, responders);
                                                    crate::log!(warn, "no responder found for id: {:p} process: {} thread: {:?}", std::sync::Arc::as_ptr(&responders), std::process::id(), std::thread::current().id());
                                                }
                                            }
                                            moredata.clear();
                                        }
                                    }
                                    else {
                                        moredata.extend_from_slice(&buffer[..final_bytes as usize]);

                                        if moredata.is_empty() {
                                            crate::log!(error, "readfile buffer is empty");
                                        }
                                        else {
                                            let output = String::from_utf8_lossy(&moredata);

                                            let (id, command, args) = parse_mirror_command(&output);
                                            crate::log!(debug, "receive pipe sysycall id: {} command: {}", id, command);
                                            let option = { responders.lock().unwrap().remove(&id) };
                                            if let Some(responder) = option {
                                                if let Err(e) = responder.send(args) {
                                                    crate::log!(info, "failed to send virtual syscall: {:?}", e);
                                                }
                                            }
                                            else {
                                                crate::log!(warn, "no responder found for id: {} syscall: {} responders: {:?}", id, command, responders);
                                                crate::log!(warn, "no responder found for id: {:p} process: {} thread: {:?}", std::sync::Arc::as_ptr(&responders), std::process::id(), std::thread::current().id());
                                            }
                                        }
                                        moredata.clear();
                                    }
                                }
                                else {
                                    crate::log!(error, "readfile failed, error code: {}, message: {}", err, tools::utils::get_winapi_error_message(err));
                                    break;
                                }
                            }
                            else {
                                if bytes == buffer.len() as u32 {
                                    if buffer[bytes as usize - 1] == b'}' {
                                        moredata.extend_from_slice(&buffer[..bytes as usize]);
                                        
                                        if moredata.is_empty() {
                                            crate::log!(error, "readfile buffer is empty");
                                        }
                                        else {
                                            let output = String::from_utf8_lossy(&moredata);

                                            let (id, command, args) = parse_mirror_command(&output);
                                            crate::log!(debug, "receive pipe sysycall id: {} command: {}", id, command);
                                            let option = { responders.lock().unwrap().remove(&id) };
                                            if let Some(responder) = option {
                                                if let Err(e) = responder.send(args) {
                                                    crate::log!(info, "failed to send virtual syscall: {:?}", e);
                                                }
                                            }
                                            else {
                                                crate::log!(warn, "no responder found for id: {} syscall: {} responders: {:?}", id, command, responders);
                                                crate::log!(warn, "no responder found for id: {:p} process: {} thread: {:?}", std::sync::Arc::as_ptr(&responders), std::process::id(), std::thread::current().id());
                                            }
                                        }
                                        moredata.clear();
                                    }
                                    else {
                                        moredata.extend_from_slice(&buffer[..bytes as usize]);
                                    }
                                }
                                else {
                                    moredata.extend_from_slice(&buffer[..bytes as usize]);

                                    let output = String::from_utf8_lossy(&moredata);

                                    if output.trim().is_empty() || output.chars().all(|c| c == '\0') {
                                        crate::log!(error, "readfile buffer is empty");
                                    }
                                    else {
                                        let (id, command, args) = parse_mirror_command(&output);
                                        crate::log!(debug, "receive pipe sysycall id: {} command: {}", id, command);
                                        let option = { responders.lock().unwrap().remove(&id) };
                                        if let Some(responder) = option {
                                            if let Err(e) = responder.send(args) {
                                                crate::log!(info, "failed to send virtual syscall: {:?}", e);
                                            }
                                        }
                                        else {
                                            crate::log!(warn, "no responder found for id: {} syscall: {} responders: {:?}", id, command, responders);
                                            crate::log!(warn, "no responder found for id: {:p} process: {} thread: {:?}", std::sync::Arc::as_ptr(&responders), std::process::id(), std::thread::current().id());
                                        }
                                    }
                                    moredata.clear();
                                }
                            }
                        }
                        crate::log!(info, "response virtual syscall named pipe reader closed.");
                    });

                    //send virtual syscall mpsc message to cocrew by named pipe
                    let mut rx = {REDIRECT_SYS_CALL_CHANNEL.rx.lock().unwrap().take().unwrap()};
                    loop {
                        let mirror_sys_call = rx.blocking_recv();
                        match mirror_sys_call {
                            Some(mirror_call) => {
                            
                                let formatted_call = format_mirror_syscall(&mirror_call);

                                let responder = mirror_call.responder;
                                {
                                    responders_.lock().unwrap().insert(mirror_call.cid, responder);
                                }

                                let event =  win::System::Threading::CreateEventW(
                                    std::ptr::null_mut(),
                                    win::Foundation::TRUE, 
                                    win::Foundation::FALSE,
                                    std::ptr::null_mut()
                                );

                                let mut overlapped: win::System::IO::OVERLAPPED = std::mem::zeroed();
                                overlapped.hEvent = event;

                                let mut bytes: u32 = 0;
                                let result = win::Storage::FileSystem::WriteFile(
                                    pipe_handle_.get().to_owned(),
                                    formatted_call.as_bytes().as_ptr(),
                                    formatted_call.len() as u32,
                                    &mut bytes,
                                    &mut overlapped
                                );
                                
                                let _eg = EventGuard(event);

                                if result == win::Foundation::FALSE {
                                    let error = win::Foundation::GetLastError();
                                    if error == win::Foundation::ERROR_IO_PENDING {
                                        win::System::Threading::WaitForSingleObject(event, win::System::Threading::INFINITE);
                                        
                                        let mut final_bytes: u32 = 0;
                                        let ret = win::System::IO::GetOverlappedResult(
                                            pipe_handle_.get().to_owned(),
                                            &mut overlapped,
                                            &mut final_bytes,
                                            win::Foundation::FALSE
                                        );

                                        if ret == win::Foundation::FALSE {
                                            if let Some(responder) = responders_.lock().unwrap().remove(&mirror_call.cid) {
                                                responder.send(std::collections::HashMap::new()).unwrap_or_else(|err| {
                                                    crate::log!(error, "send empty result to virtual syscall failed. {:?}", err);
                                                });
                                            }
                                        }
                                        else {
                                            //crate::log!(warn, "send format virtual syscall to namedpipe success. {:p} process: {} thread: {:?}", std::sync::Arc::as_ptr(&responders_), std::process::id(), std::thread::current().id());
                                        }
                                    }
                                    else {
                                        crate::log!(error, "send virtual syscall to namedpipe error, failed code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                                        break;
                                    }
                                }
                                else {
                                    //crate::log!(warn, "write virtual syscall to namedpipe success. {:p} process: {} thread: {:?}", std::sync::Arc::as_ptr(&responders_), std::process::id(), std::thread::current().id());
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    };

                    rx.close();
                    win::Foundation::CloseHandle(pipe_handle_.get().to_owned());
                    log!(error, "redirect_syscall_2_cocrew named pipe writer closed.");
                    break;
                }
                else {
                    let error = win::Foundation::GetLastError();
                    
                    if error == win::Foundation::ERROR_PIPE_BUSY || error == win::Foundation::ERROR_FILE_NOT_FOUND {
                        crate::log!(info, "redirect_syscall_2_cocrew CreateFileW wait and retry, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    else {
                        //println!("CreateFileW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                        crate::log!(error, "redirect_syscall_2_cocrew CreateFileW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                        break;
                    }
                }
            }
        }
        return ();
    });
}

pub fn async_connect_syscall_namedpipe() {
    unsafe {
        redirect_syscall_2_cocrew();
    }
}

fn format_mirror_syscall(call: &MirrorSysCall) -> String {
    let str = format!("{{\"cid\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
            call.cid,
            call.api,
            call.args.iter()
                .map(|(k, v)| format!("\"{}\": {:?}", k, v))
                .collect::<Vec<_>>()
                .join(", "));
    return str;
}

pub fn parse_mirror_command(json: &str) -> (u32, String, std::collections::HashMap<String, String>) {
    let json = json.trim();

    let cid = extract_number_field(json, "cid").unwrap_or_else(|err| {
        crate::log!(error, "parse cid field failed: {}", err);
        0
    });

    let api = extract_string_field(json, "api").unwrap_or_else(|err|{
        crate::log!(error, "parse api field failed: {}", err);
        "".to_string()
    });

    let args = extract_args_field(json).unwrap_or_else(|err| {
        crate::log!(error, "parse args field failed: {}", err);
        std::collections::HashMap::new()
    });

    return (cid, api, args);
}

fn extract_number_field(json: &str, field_name: &str) -> Result<u32, String> {
    let pattern = format!(r#""{}": "#, field_name);
    let start = json.find(&pattern)
        .ok_or_else(|| format!("Field '{}' not found", field_name))?;
    
    let value_start = start + pattern.len();
    let mut value_end = value_start;
    
    let chars: Vec<char> = json.chars().collect();
    while value_end < chars.len() {
        let ch = chars[value_end];
        if ch.is_ascii_digit() {
            value_end += 1;
        } 
        else {
            break;
        }
    }
    
    let number_str = &json[value_start..value_end];
    number_str.parse::<u32>()
        .map_err(|_| format!("invalid number format for field '{}'", field_name))
}
    
fn extract_string_field(json: &str, field_name: &str) -> Result<String, String> {
    let pattern = format!(r#""{}": ""#, field_name);
    let start = json.find(&pattern)
        .ok_or_else(|| format!("field '{}' not found", field_name))?;
    
    let value_start = start + pattern.len();
    let value_end = json[value_start..]
        .find('"')
        .ok_or_else(|| format!("string end not found for field '{}'", field_name))?;
    
    Ok(json[value_start..value_start + value_end].to_string())
}
    
fn extract_args_field(json: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let pattern = r#""args": {"#;
    let start = json.find(pattern)
        .ok_or_else(|| "args field not found".to_string())?;
    
    let args_start = start + pattern.len();
    
    let mut brace_count = 1;
    let mut args_end = args_start;
    let chars: Vec<char> = json.chars().collect();
    
    while args_end < chars.len() && brace_count > 0 {
        match chars[args_end] {
            '{' => brace_count += 1,
            '}' => brace_count -= 1,
            _ => {}
        }
        args_end += 1;
    }
    
    if brace_count != 0 {
        return Err("Malformed args object".to_string());
    }
    
    let args_content = &json[args_start..args_end - 1];
    parse_key_value_pairs(args_content)
}

fn parse_key_value_pairs(content: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let mut args = std::collections::HashMap::new();
    
    if content.trim().is_empty() {
        return Ok(args);
    }
    
    let pairs: Vec<&str> = content.split(',').collect();
    
    for pair in pairs {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        
        let colon_pos = pair.find(':')
            .ok_or_else(|| "Invalid key-value pair format".to_string())?;
        
        let key_part = pair[..colon_pos].trim();
        let value_part = pair[colon_pos + 1..].trim();
        
        let key = strip_quotes(key_part)?;
        let value = strip_quotes(value_part)?;
        
        args.insert(key, value);
    }
    
    Ok(args)
}
    
fn strip_quotes(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return Err("String must be quoted".to_string());
    }
    Ok(s[1..s.len() - 1].to_string())
}


#[cfg(test)]
mod tests {
    use crate::syscallredirect::parse_mirror_command;

    #[test]
    fn parse_mirror_command_test() {
        let json = r#"{"id": 0,"command": "NtQueryDirectoryFile","args": {"fileinformation": "mocs_compilation_Debug.obj", "key": "value"}}"#;
        let (id, command, args) = parse_mirror_command(json);
        println!("id: {}, command: {}, args: {:?}", id, command, args);
    }
}