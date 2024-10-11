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
                    r#"{{"compiler_path_or_arch":"{}", "compiler_working_dir":"{}", "compiler_commands":"{:?}", "build_and_compiler_type":"{}"}}"#,
                    compiler_input.compiler_path_or_arch.to_string_lossy(), 
                    compiler_input.compiler_working_dir.to_string_lossy(), 
                    commands,
                    compiler_input.build_and_compiler_type.to_string_lossy()
                );

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
