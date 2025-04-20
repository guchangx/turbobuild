use std::io::{BufRead, Read, Write};
use std::net::ToSocketAddrs;

pub struct SocketClient {
}

impl SocketClient {

    pub fn new() -> Self {
        Self {
        
        }
    }

    pub fn request_compile(&self, compiler_input: crate::commands::CompilerInput) -> Result<i32, ()> {

        let process_id = std::process::id();
        let thread_id = std::thread::current().id();
        
        let addr = "localhost:22403".to_socket_addrs().unwrap().next().unwrap();
        
        match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(1)) {
            Ok(mut stream) => {
                stream.set_nodelay(true).unwrap();
                stream.set_write_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                let commands: Vec<_> = compiler_input.compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();

                let buffer = format!(
                    r#"{{"project": {:?}, "compiler_path": {:?}, "compiler_working_dir": {:?}, "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
                    compiler_input.project.to_string_lossy(),
                    compiler_input.compiler_path.to_string_lossy(),
                    compiler_input.compiler_working_dir.to_string_lossy(),
                    commands,
                    compiler_input.build_and_compiler_type.to_string_lossy()
                );

                println!("{} send to turbobuild: {}, task id: {}-{:?}", current_datetime(), buffer, process_id, thread_id);
                let _size = stream.write(buffer.as_bytes());
                stream.flush().unwrap();
                let mut buffer = [0 as u8; 128];
                stream.set_read_timeout(Some(std::time::Duration::from_secs(360))).unwrap();
                loop {
                    match stream.read(&mut buffer) {
                        Ok(size) => {
                            if size == 0 {
                                println!("{} read form turbobuild done. task id: {}-{:?}", current_datetime(), process_id, thread_id);
                                break;
                            }
                            else if size < buffer.len() {
                                buffer[0..size].lines().for_each(|line| {
                                    println!("{} read form turbobuild: {}", current_datetime(), line.unwrap());
                                });
                            }
                        },
                        Err(err) => {
                            println!("{} read from turbobuild server failed: {:?}. task id: {}-{:?}", current_datetime(), err, process_id, thread_id);
                            break;
                        }
                    }
                }
                return Ok(0);
            },
            Err(err) => {
                let kind = err.kind();
                let message = err.to_string();
                println!("buildassist connect {} failed: {:?}, {}. task id: {}-{:?}", addr, kind, message, process_id, thread_id);
                return Err(());
            },
        }
    }
}

pub fn current_datetime() -> String {
    let total_seconds = std::time::SystemTime::now()
        .duration_since( std::time::UNIX_EPOCH)
        .expect("fetch current time failed.")
        .as_secs();
   
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 3600) % 24;
    //let days = total_seconds / 86400;
    //let year = 1970 + days / 365;
    //let month = (days % 365) / 30 + 1;
    //let day = (days % 365) % 30 + 1;
    return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
}

#[cfg(test)]
mod tests {

    #[test]
    //cargo test --package buildassist --tests serde_json_2_struct -- --show-output
    fn serde_json_2_struct() {
        
        let compiler_commands = vec![std::ffi::OsString::from("/test")];
        let commands: Vec<_> = compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();
        
        let data = format!(
            r#"{{"project": "{}", "compiler_path": "{}", "compiler_working_dir": "{}", "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
            std::ffi::OsString::from("draft").to_string_lossy(),
            std::ffi::OsString::from(r#"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe"#).to_string_lossy(),
            std::ffi::OsString::new().to_string_lossy(),
            commands,
            std::ffi::OsString::new().to_string_lossy()
        );
        
        println!("socket data {:?}", data);

        let input: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert!(input["compiler_path"].as_str().is_some());
        assert!(input["compiler_working_dir"].as_str().is_some());
        assert!(input["compiler_commands"].as_array().is_some());
        assert!(input["build_and_compiler_type"].as_str().is_some());
    }
}