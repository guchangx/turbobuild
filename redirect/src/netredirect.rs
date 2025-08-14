use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;

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

    let _ = crate::RUNTIME.lock().unwrap().spawn(async move {
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
    
    let _ = crate::RUNTIME.lock().unwrap().spawn(async move {
        let mut rx = NET_REDIRECT_CHANNEL.rx.lock().unwrap().take().unwrap();
        while let Some(mirror_cmd) = rx.recv().await {
            let formatted_command = format_mirror_command(&mirror_cmd);

            match writer.write(formatted_command.as_bytes()).await {
                Ok(size) => {
                    responders_.lock().unwrap().insert(mirror_cmd.id, mirror_cmd.responder);
                    crate::log!(info, "sent virtual command: {} with id: {}", mirror_cmd.command, mirror_cmd.id);
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
}

pub fn async_connect_named_pipe() {
    crate::RUNTIME.lock().unwrap().spawn(async move {
        crate::netredirect::connect_named_pipe().await;
    });
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