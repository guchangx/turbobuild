use std::io::{BufRead, Read, Write};
use std::net::ToSocketAddrs;

pub struct SocketClient {
}

impl SocketClient {

    pub fn new() -> Self {
        Self {
        
        }
    }
    //TODO: should send compile failed message to IDE
    //The VS format of the output should be:
    //{ filename(line-number [, column-number]) | tool-name } : [ any-text ] {error | warning} code-type-and-number : localizable-string [ any-text ]
    //Where:
    //{ a | b } is a choice of either a or b,
    //[ item ] is an optional string or parameter,
    //text represents a literal.
    //For example:
    //C:\sourcefile.cpp(134) : error C2143: syntax error : missing ';' before '}'
    //LINK : fatal error LNK1104: cannot open file 'some-library.lib'
    pub fn request_compile(&self, compiler_input: crate::commands::CompilerInput) -> Result<i32, ()> {

        let process_id = std::process::id();
        let thread_id = std::thread::current().id();
        
        let addr = "localhost:22403".to_socket_addrs().unwrap().next().unwrap();
        
        match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3)) {
            Ok(mut stream) => {
                stream.set_nodelay(true).unwrap();
                stream.set_write_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(360))).unwrap();

                let commands: Vec<_> = compiler_input.compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();
    
                let buffer = format!(
                    r#"{{"solution": {:?}, "project": {:?}, "compiler": {:?}, "working_dir": {:?}, "commands": {:?}, "type": "{}"}}"#,
                    compiler_input.solution.to_string_lossy(),
                    compiler_input.project.to_string_lossy(),
                    compiler_input.compiler_path.to_string_lossy(),
                    compiler_input.compiler_working_dir.to_string_lossy(),
                    commands,
                    compiler_input.build_and_compiler_type.to_string_lossy()
                );

                println!("{} send to turbobuild: {}, task id: {}-{:?}", current_datetime(), buffer, process_id, thread_id);
                let _size = stream.write_all(buffer.as_bytes());
                stream.flush().unwrap();

                let mut data = Vec::new();
                let mut ret = Ok(0);
                loop {
                    let mut buffer = [0 as u8; 256];
                    match stream.read(&mut buffer) {
                        Ok(size) => {
                            if size == 0 {
                                data.clear();
                                println!("{} read form turbobuild done.", current_datetime());
                                break;
                            }
                            else if size == buffer.len() {
                                data.extend_from_slice(&buffer);
                            }
                            else if size < buffer.len() {
                                if data.is_empty() {     
                                    buffer[0..size].lines().for_each(|line| {
                                        if let Ok(file) = line {
                                            if file.trim_end().ends_with(".i") || file.trim_end().ends_with(".cpp") || file.trim_end().ends_with(".cc") || file.trim_end().ends_with(".cxx") {
                                                println!("{} read form turbobuild: {}", current_datetime(), file);
                                            }
                                            else if file.eq("waiting...") {
                                                println!("{} read form turbobuild: {}", current_datetime(), file);
                                            }
                                            else {
                                                ret = Err(());
                                                eprintln!("{} read error form turbobuild: {}", current_datetime(), file);
                                            }
                                        }
                                    });   
                                }
                                else {
                                    data.extend_from_slice(&buffer[0..size]);
                                    data[..].lines().for_each(|line| {
                                        if let Ok(file) = line {
                                            if file.trim_end().ends_with(".i") || file.trim_end().ends_with(".cpp") || file.trim_end().ends_with(".cc") || file.trim_end().ends_with(".cxx") {
                                                println!("{} read form turbobuild: {}", current_datetime(), file);
                                            }
                                            else if file.eq("waiting...") {
                                                println!("{} read form turbobuild: {}", current_datetime(), file);
                                            }
                                            else {
                                                //fatal error
                                                ret = Err(());
                                                eprintln!("{} read error form turbobuild: {}", current_datetime(), file);
                                            }
                                        }
                                    });
                                }
                            }
                        },
                        Err(err) => {
                            if err.kind() == std::io::ErrorKind::TimedOut {
                                println!("{} read from turbobuild server timeout value {:?}.", current_datetime(), stream.read_timeout());
                            }
                            println!("{} read from turbobuild server failed: {:?}. task id: {}-{:?}", current_datetime(), err, process_id, thread_id);
                            ret = Err(());
                            break;
                        }
                    }
                }
                stream.shutdown(std::net::Shutdown::Both).unwrap();
                return ret;
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
    let since = std::time::SystemTime::now()
        .duration_since( std::time::UNIX_EPOCH)
        .expect("fetch current time failed.");
    
    let milliseconds = since.subsec_nanos() / 1_000_000;
    let total_seconds = since.as_secs();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 3600) % 24;

    return format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, milliseconds);
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