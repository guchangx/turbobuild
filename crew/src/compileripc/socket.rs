
use std::io::Read;
use std::io::Write;

#[derive(Default, Clone)]
pub struct Receiver {
    port: u16,
    common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>,
    distributor: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>,
    work_env: crate::platform::windows::WindowsCompilerEnv,
}

impl Receiver {
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>, distributor: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>) -> Self {
        return Self {
            port: 9301,
            common,
            distributor,
            work_env: crate::platform::windows::WindowsCompilerEnv::default()
        }
    } 
    
    pub fn init(&self) {
        
        let listener = std::net::TcpListener::bind(format!("localhost:{}", self.port)).unwrap();
        
        log::info!("init ipc socket {}", listener.local_addr().unwrap());
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    log::debug!("new buildassist connection socket");
                    let runtime = self.common.upgrade().unwrap()
                        .lock().unwrap()
                        .pool.clone().unwrap();
                    
                    let distributor = self.distributor.clone();
                    let runtime_ = runtime.clone();
                    let env = self.work_env.clone();
                    let _ = runtime.spawn(async {
                        Self::handle_ipc_stream(stream, runtime_, env, distributor).await;
                    });
                    //can't block current run.
                },
                Err(err) => {
                    log::error!("tcplistener bind error: {}", err);
                }
            }
        }
    }
    
    async fn handle_ipc_stream(mut stream: std::net::TcpStream, runtime: std::sync::Arc<tokio::runtime::Handle>, work_env: crate::platform::windows::WindowsCompilerEnv, distor: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>) {
       
        let mut data = "".to_string();
        let mut buffer = [0 as u8; 256];
        loop {
            match stream.read(&mut buffer) {
                Ok(size) => {
                    data = data + std::str::from_utf8(&buffer[..size]).unwrap();
                    if size < buffer.len() {
                        log::debug!("buildassist connection data {:?}", data);
                        
                        let input: serde_json::Value = serde_json::from_str(data.as_str()).unwrap();

                        let compiler = input["compiler_path"].as_str().unwrap();
                        let working = input["compiler_working_dir"].as_str().unwrap();
                        let commands = input["compiler_commands"].as_array().unwrap();
                        let r#type = input["build_and_compiler_type"].as_str().unwrap();
                        
                        let input = crate::compiler::model::CompilerInput {
                            compiler_path: std::ffi::OsString::from(compiler),
                            compiler_working_dir: std::ffi::OsString::from(working),
                            compiler_commands: commands.iter().map(|item| item.as_str().unwrap().into()).collect(),
                            build_and_compiler_type: std::ffi::OsString::from(r#type),
                        };

                        crate::compiler::interface::request_compile(input, runtime.clone(), if work_env.winkits_includes_path.is_empty() {Some(work_env)} else { None }, distor).await;                        
                        stream.write_all(b"done").unwrap();
                        break;
                    }
                },
                Err(err) => {
                    println!("buildassist connect data error: {}", err);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    //cargo test --package crew --tests serde_json_2_struct -- --show-output
    fn serde_json_2_struct() {
        
        let compiler_commands = vec![std::ffi::OsString::from("")];
        let commands: Vec<_> = compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();

        let data = format!(
            r#"{{"compiler_path": "{}", "compiler_working_dir": "{}", "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
            "",
            "",
            commands,
            ""
        );

        let input: serde_json::Value = serde_json::from_str(data.as_str()).unwrap();
        assert!(input["compiler_path"].as_str().is_some());
    }
}