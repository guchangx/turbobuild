use std::{any::Any, error::Error, io::{ErrorKind, Read, Write}};


#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct CompileOutput {
    compiled_filename: Vec<std::ffi::OsString>,
    compile_status: bool,
    compile_output: std::ffi::OsString,
}

pub struct NetworkClient {
    client: reqwest::blocking::Client,
}

impl NetworkClient {

    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(core::time::Duration::from_secs(360))
            .build()
            .expect("bulid a request client failed.");
        
        Self {
            client,
        }
    }

    fn post(&self, route: String, input: crate::commands::CompilerInput) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let base = reqwest::Url::parse("http://127.0.0.1:9302/").unwrap();
        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&input)
        .send();
        return response;
    }

    pub fn request_local_compile(&self, compiler_input: crate::commands::CompilerInput) {
        let response = self.post("requestcompile".to_string(), compiler_input);
        match response {
            Ok(response) => {
                let compile_output: Result<CompileOutput, reqwest::Error> = response.json();
                match compile_output {
                    Ok(output) => {
                        if output.compile_status {
                            for line in output.compile_output.to_string_lossy().lines()
                            {
                                println!("done: {:?}.", line);
                            }
                        }
                        else {
                            for line in output.compile_output.to_string_lossy().lines()
                            {
                                println!("error: {:?}", line);
                            }
                        }
                    },
                    Err(error) => {
                        println!("parse response json failed. {:?}.", error);
                    },
                }
            }
            Err(error) => {
                println!("assist request local compile failed. {:?}", error);
            }
        }
    }

    pub fn request_compile(&self, compiler_input: crate::commands::CompilerInput) {
        match std::net::TcpStream::connect("localhost:9301") {
            Ok(mut stream) => {
                println!("connect to turbobuild server success.");
                
                let commands:Vec<_> = compiler_input.compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();
                let buffer = format!(r#"{{
                "compiler_path_or_arch":"{}",
                "compiler_working_dir":"{}",
                "compiler_commands":"{:?}",
                "build_and_compiler_type":"{}"
                }}"#,
                
                compiler_input.compiler_path_or_arch.to_string_lossy(), 
                compiler_input.compiler_working_dir.to_string_lossy(), 
                commands, 
                compiler_input.build_and_compiler_type.to_string_lossy());

                stream.write(buffer.as_bytes()).unwrap();

                let mut data = [0 as u8; 128];
                match stream.read(&mut data) {
                    Ok(_size) => {
                        println!("read from turbobuild server: {:?}", data);
                    },
                    Err(err) => {
                        println!("read from turbobuild server failed. {:?}", err);
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
