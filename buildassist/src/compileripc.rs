use std::io::{Read, Write};
struct CompilerOutput {
    compiled_filename: Vec<std::ffi::OsString>,
    compile_status: bool,
    compile_output: std::ffi::OsString,
}

pub struct SocketClient {
}

impl SocketClient {

    pub fn new() -> Self {
        Self {
        
        }
    }

    pub fn request_compile(&self, compiler_input: crate::commands::CompilerInput) {
        match std::net::TcpStream::connect("localhost:9301") {
            Ok(mut stream) => {
                println!("connect to turbobuild server success.");
                
                let commands: Vec<_> = compiler_input.compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();

                let buffer = format!(
                    r#"{{"project": {:?}, "compiler_path": {:?}, "compiler_working_dir": {:?}, "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
                    compiler_input.project.to_string_lossy(),
                    compiler_input.compiler_path.to_string_lossy(),
                    compiler_input.compiler_working_dir.to_string_lossy(),
                    commands,
                    compiler_input.build_and_compiler_type.to_string_lossy()
                );
                
                println!("send to turbobuild: {}", buffer);

                stream.write(buffer.as_bytes()).unwrap();

                let mut data = String::new();
                let mut buffer = [0 as u8; 128];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(size) => {
                            data = data + std::str::from_utf8(&buffer[0..size]).unwrap();
                            if size < buffer.len() {
                                
                                println!("read form turbobuild: {}", data);
                                break;
                            }
                        },
                        Err(err) => {
                            println!("read from turbobuild server failed. {:?}", err);
                            break;
                        }
                    }
                }
            },
            Err(err) => {
                let kind = err.kind();
                let message = err.to_string();
                
                println!("buildassist failed: {:?}, {}", kind, message);
            },
        }
    }
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