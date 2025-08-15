use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;

async fn connect() -> std::result::Result<tokio::net::windows::named_pipe::NamedPipeClient, std::io::Error> {
    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";
    let mut client = loop {
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

pub struct MirrorCommand {
    pub id: u32,
    pub command: String,
    pub args: std::collections::HashMap<String, String>,
    pub responder: tokio::sync::oneshot::Sender<std::collections::HashMap<String, String>>,
}

pub struct Channel {
    pub tx: tokio::sync::mpsc::Sender<MirrorCommand>,
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<MirrorCommand>>>,
}

pub static NET_REDIRECT_CHANNEL: std::sync::LazyLock<Channel> = std::sync::LazyLock::new(|| {
    let (tx, rx) = tokio::sync::mpsc::channel::<MirrorCommand>(512);
    let channel = Channel { tx, rx: std::sync::Mutex::new(Some(rx)) };
    return channel;
});

pub async fn connect_named_pipe() {
    let client = connect().await.unwrap();
    let (mut reader, mut writer) = tokio::io::split(client);

    type Map = std::collections::HashMap<u32, tokio::sync::oneshot::Sender<std::collections::HashMap<String, String>>>;
    let responders: std::sync::Arc<std::sync::Mutex<Map>> = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let responders_ = responders.clone();

    let handle_r = crate::RUNTIME.lock().unwrap().spawn(async move {
        let mut data = vec![0; 1024];
        loop {
            match reader.read(&mut data).await {
                Ok(size) if size > 0 => {
                    let command = String::from_utf8_lossy(&data[..size]);
                    crate::log!(info, "received virtual command: {}", command);

                    let (id, command, args) = parse_mirror_command(&command);
                    if let Some(responder) = responders.lock().unwrap().remove(&id) {
                        crate::log!(info, "responding to command: {} with id: {}", command, id); 
                        if let Err(e) = responder.send(args) {
                            crate::log!(info, "failed to send virtual command: {:?}", e);
                        }
                    } 
                    else {
                        crate::log!(warn, "no responder found for id: {} command: {}", id, command);
                    }
                }
                Ok(_) => break,
                Err(e) => {
                    let err = format!("failed to read from pipe: {}", e);
                    crate::log!(error, "{}", err);
                    break;
                }
            }
        }
        crate::log!(info, "virtual command named pipe reader closed.");
    });

    let handle_w = crate::RUNTIME.lock().unwrap().spawn(async move {
        let mut rx = NET_REDIRECT_CHANNEL.rx.lock().unwrap().take().unwrap();
        while let Some(mirror_cmd) = rx.recv().await {
            let formatted_command = format_mirror_command(&mirror_cmd);

            match writer.write(formatted_command.as_bytes()).await {
                Ok(size) => {
                    writer.flush().await.unwrap();
                    responders_.lock().unwrap().insert(mirror_cmd.id, mirror_cmd.responder);
                    crate::log!(info, "sent virtual command: {} with id: {} size: {}", mirror_cmd.command, mirror_cmd.id, size);
                },
                Err(e) => {
                    let err = format!("failed to write to pipe: {}", e);
                    crate::log!(error, "{}", err);
                }
            }
        }
        crate::log!(info, "virtual command named pipe writer closed.");
        writer.shutdown().await.unwrap();
    });

    crate::log!(info, "virtual command named pipe disconnected.");

    let _ = tokio::join!(handle_r, handle_w);
}

unsafe fn redirect_command_2_cocrew() {

    use std::os::windows::ffi::OsStrExt;
    let iocp = winapi::um::ioapiset::CreateIoCompletionPort(
        winapi::um::handleapi::INVALID_HANDLE_VALUE,
        std::ptr::null_mut(),
        0,
        0
    );

    let iocp_handle = tools::ptr::HandleBox::new(iocp);
    let iocp_handle_ = iocp_handle.clone();

    let _ = crate::RUNTIME.lock().unwrap().spawn(async move {

        let name = std::ffi::OsString::from("\\\\.\\pipe\\os_operate_request_pipe");
        let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    
        let mut rx = NET_REDIRECT_CHANNEL.rx.lock().unwrap().take().unwrap();
        
        type Map = std::collections::HashMap<u32, tokio::sync::oneshot::Sender<std::collections::HashMap<String, String>>>;
        let responders: std::sync::Arc<std::sync::Mutex<Map>> = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

        if winapi::um::namedpipeapi::WaitNamedPipeW(name.as_ptr(), 300) == winapi::shared::minwindef::TRUE {

            for _ in 0..3 {
                let pipe_handle = winapi::um::fileapi::CreateFileW(name.as_ptr(), winapi::um::winnt::GENERIC_WRITE,
                    0,
                    std::ptr::null_mut(),  
                    winapi::um::fileapi::OPEN_EXISTING, 
                    winapi::um::winbase::FILE_FLAG_OVERLAPPED, 
                    winapi::shared::ntdef::NULL
                );

                if !pipe_handle.is_null() && pipe_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {

                    let pipe_handle = tools::ptr::HandleBox::new(pipe_handle);

                    if winapi::um::ioapiset::CreateIoCompletionPort(
                        pipe_handle.get().to_owned(),
                        iocp_handle.get().to_owned(),
                        0,
                        0
                    ).is_null() {
                        winapi::um::handleapi::CloseHandle(iocp_handle.get().to_owned());
                        winapi::um::handleapi::CloseHandle(pipe_handle.get().to_owned());
                        println!("CreateIoCompletionPort failed, error code: {}, message: {}", winapi::um::errhandlingapi::GetLastError(), tools::utils::get_winapi_error_message(winapi::um::errhandlingapi::GetLastError()));
                        break;
                    }

                    let pipe_handle_ = pipe_handle.clone();
                    let responders_ = responders.clone();
                    let _handle_r = crate::RUNTIME.lock().unwrap().spawn(async move {
                        loop {

                            let mut buffer = vec![0u8; 512];
                            let mut bytes: winapi::shared::minwindef::DWORD = 0;

                            let result = winapi::um::fileapi::ReadFile(
                                pipe_handle_.get().to_owned(),
                                buffer.as_mut_ptr() as *mut _,
                                512,
                                &mut bytes,
                                std::ptr::null_mut()
                            );

                            crate::log!(info, "read virtual command named pipe size: {}.", bytes);

                            if result == winapi::shared::minwindef::FALSE {
                                let err = winapi::um::errhandlingapi::GetLastError();
                                println!("ReadFile failed, error code: {}, message: {}", err, tools::utils::get_winapi_error_message(err));
                                if err == winapi::shared::winerror::ERROR_IO_PENDING {
                                    continue;
                                }
                                else if err == winapi::shared::winerror::ERROR_MORE_DATA {
                                    continue;
                                }
                                else if err == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                    break;
                                }
                                else
                                {
                                    break;
                                }
                            }
                            else {
                                let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                                let (id, command, args) = parse_mirror_command(&output);
                                if let Some(responder) = responders.lock().unwrap().remove(&id) {
                                    crate::log!(info, "responding to command: {} with id: {}", command, id); 
                                    if let Err(e) = responder.send(args) {
                                        crate::log!(info, "failed to send virtual command: {:?}", e);
                                    }
                                } 
                                else {
                                    crate::log!(warn, "no responder found for id: {} command: {}", id, command);
                                }
                            }
                        }

                    });

                    loop {
                        let mirror_cmd = rx.recv().await;
                        match mirror_cmd {
                            Some(mirror_cmd) => {

                                let formatted_command = format_mirror_command(&mirror_cmd);

                                let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
                                let mut bytes: winapi::shared::minwindef::DWORD = 0;
                                let result = winapi::um::fileapi::WriteFile(
                                    pipe_handle.get().to_owned(),
                                    formatted_command.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
                                    formatted_command.len() as u32,
                                    &mut bytes,
                                    &mut overlapped
                                );

                                if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                                    let error = winapi::um::errhandlingapi::GetLastError();
                                    if error == winapi::shared::winerror::ERROR_IO_PENDING {
                                        
                                    }
                                    else if error == winapi::shared::winerror::ERROR_BROKEN_PIPE || error == winapi::shared::winerror::ERROR_NO_DATA {
                                        //break;
                                    }
                                    else {
                                        println!("write pipe error, failed code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                                        crate::logger::output_debug_string(&format!("write pipe error, failed code: {}, message: {}", error, tools::utils::get_winapi_error_message(error)));
                                        break;
                                    }
                                }
                                else {
                                    responders_.lock().unwrap().insert(mirror_cmd.id, mirror_cmd.responder);
                                }
                                
                                //if winapi::shared::minwindef::FALSE == winapi::um::fileapi::FlushFileBuffers(pipe_handle.get().to_owned()) {
                                //    println!("FlushFileBuffers failed, error code: {}, message: {}", winapi::um::errhandlingapi::GetLastError(), tools::utils::get_winapi_error_message(winapi::um::errhandlingapi::GetLastError()));
                                //}
                            },
                            None => {
                                break;
                            }
                        }
                    };
                    
                    winapi::um::ioapiset::PostQueuedCompletionStatus(
                        iocp_handle.get().to_owned(), 
                        0, 
                        0, 
                        std::ptr::null_mut()
                    );

                    winapi::um::handleapi::CloseHandle(pipe_handle.get().to_owned());
                    break;
                }
                else {
                    let error = winapi::um::errhandlingapi::GetLastError();
                    
                    if error == winapi::shared::winerror::ERROR_PIPE_BUSY || error == winapi::shared::winerror::ERROR_FILE_NOT_FOUND {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                    else {
                        println!("CreateFileW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                    }
                }
            }
        }
        else {
            let error = winapi::um::errhandlingapi::GetLastError();
            println!("WaitNamedPipeW failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
        }
        
        loop {
            match rx.try_recv() {
                Ok(_) => {
                },
                Err(err) => {
                    crate::logger::output_debug_string(&format!("redirect_stdout_log_2_cocrew: failed to receive message: {}", err));
                    break;
                },
            }
        }
        rx.close();
        
        return ();
    });

    let _ = std::thread::spawn(move || {
        crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus start."));
        loop {
            let mut bytes: winapi::shared::minwindef::DWORD = 0;
            let mut key: usize = 0;
            let mut overlapped: *mut winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();

            let result = winapi::um::ioapiset::GetQueuedCompletionStatus(
                iocp_handle_.get().to_owned(),
                &mut bytes,
                &mut key,
                &mut overlapped,
                winapi::um::winbase::INFINITE
            );

            if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                let error = winapi::um::errhandlingapi::GetLastError();
                println!("GetQueuedCompletionStatus failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
                break;
            }
            else {
                if key == 0 {
                    break;
                }
            }
        }
        winapi::um::handleapi::CloseHandle(iocp_handle_.get().to_owned());
        crate::logger::output_debug_string(&format!("GetQueuedCompletionStatus end."));
    });
}

pub fn async_connect_named_pipe() {
    unsafe {
        redirect_command_2_cocrew();
    }

    //crate::RUNTIME.lock().unwrap().spawn(async move {
    //    connect_named_pipe().await;
    //});
}

fn format_mirror_command(command: &MirrorCommand) -> String {
    let str = format!("{{\"id\": {}, \"command\": \"{}\", \"args\": {{{}}}}}",
                     command.id,
                     command.command,
                     command.args.iter()
                         .map(|(k, v)| format!("\"{}\": {:?}", k, v))
                         .collect::<Vec<_>>()
                         .join(", "));
    return str;
}

pub fn parse_mirror_command(json: &str) -> (u32, String, std::collections::HashMap<String, String>) {
    let json = json.trim();

    let id = extract_number_field(json, "id").unwrap();

    let command = extract_string_field(json, "command").unwrap();

    let args = extract_args_field(json).unwrap();

    return (id, command, args);
}

fn extract_number_field(json: &str, field_name: &str) -> Result<u32, String> {
    let pattern = format!(r#""{}":"#, field_name);
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
        .map_err(|_| format!("Invalid number format for field '{}'", field_name))
}
    
fn extract_string_field(json: &str, field_name: &str) -> Result<String, String> {
    let pattern = format!(r#""{}":""#, field_name);
    let start = json.find(&pattern)
        .ok_or_else(|| format!("field '{}' not found", field_name))?;
    
    let value_start = start + pattern.len();
    let value_end = json[value_start..]
        .find('"')
        .ok_or_else(|| format!("string end not found for field '{}'", field_name))?;
    
    Ok(json[value_start..value_start + value_end].to_string())
}
    
fn extract_args_field(json: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let pattern = r#""args":{"#;
    let start = json.find(pattern)
        .ok_or_else(|| "Args field not found".to_string())?;
    
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
    use crate::netredirect::parse_mirror_command;

    #[test]
    fn parse_mirror_command_test() {
        let json = r#"{"id":0,"command":"NtQueryDirectoryFile","args":{"fileinformation":".\r\n..\r\ngammaray_lz4.pdb\r\nlz4.c\r\nmocs_compilation_Debug.cpp\r\nmocs_compilation_Debug.obj\r\n"}}"#;
        let (id, command, args) = parse_mirror_command(json);
        println!("id: {}, command: {}, args: {:?}", id, command, args);
    }
}