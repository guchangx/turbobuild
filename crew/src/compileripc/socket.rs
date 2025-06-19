
use std::io::BufRead;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
            port: 22403,
            common,
            distributor,
            work_env: crate::platform::windows::WindowsCompilerEnv::default()
        }
    } 
    
    pub async fn init(&self) {
        
        let addr = format!("localhost:{}", self.port);
        log::debug!("init ipc socket {}", addr);

        let listener = tokio::net::TcpListener::bind(addr.clone()).await.unwrap();

        let runtime = self.common.upgrade().unwrap()
                .lock().unwrap()
                .pool.clone().unwrap();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let metrics = runtime.metrics(); 
            log::debug!("new buildassist connection socket. runtime: {:#?} {:#?}", metrics.num_workers(), metrics.num_alive_tasks());
            
            let distributor = self.distributor.clone();
            let runtime_ = runtime.clone();
            let env = self.work_env.clone();

            let _ =  runtime.spawn(async { Self::handle_ipc_stream(stream, runtime_, env, distributor).await;});
        }
    }
    
    async fn handle_ipc_stream(mut stream: tokio::net::TcpStream, runtime: std::sync::Arc<tokio::runtime::Handle>, work_env: crate::platform::windows::WindowsCompilerEnv, distor: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>) {
       
        let mut data = String::new();
        let mut buffer = [0 as u8; 1024];
        stream.set_nodelay(true).unwrap();
        
        loop {
            match stream.read(&mut buffer).await {
                Ok(size) => {
                    data.push_str(std::str::from_utf8(&buffer[..size]).unwrap());

                    if size < buffer.len() || buffer.ends_with(b"}") {
                        log::debug!("buildassist connection data {}", data);

                        let input: serde_json::Value = serde_json::from_str(data.as_str()).unwrap();
                        
                        let solution = input["solution"].as_str().unwrap();
                        let index = input["index"].as_str().unwrap();
                        let project = input["project"].as_str().unwrap();
                        let compiler = input["compiler_path"].as_str().unwrap();
                        let working = input["compiler_working_dir"].as_str().unwrap();
                        let commands = input["compiler_commands"].as_array().unwrap();
                        let r#type = input["build_and_compiler_type"].as_str().unwrap();
                        
                        let input = crate::compiler::model::CompilerInput {
                            solution: std::ffi::OsString::from(solution),
                            index: std::ffi::OsString::from(index),
                            project: std::ffi::OsString::from(project),
                            compiler_path: std::ffi::OsString::from(compiler),
                            compiler_working_dir: std::ffi::OsString::from(working),
                            compiler_commands: commands.iter().map(|item| item.as_str().unwrap().into()).collect(),
                            build_and_compiler_type: std::ffi::OsString::from(r#type),
                        };

                        let result = crate::compiler::interface::request_compile(input, runtime.clone(), if work_env.winkits_includes_path.is_empty() {Some(work_env)} else { None }, distor).await;  
   
                        for line in result.out.lines() {
                            let line = line.unwrap();
                            let _ = stream.write_all(line.as_bytes()).await.unwrap_or_else(|err| {
                                log::warn!("assistbuild request compile output error: {}", err);
                            });
                        };

                        let _ = stream.flush();

                        for line in result.err.lines() {
                            let line = line.unwrap();
                            let _ = stream.write_all(line.as_bytes()).await.unwrap_or_else(|err| {
                                log::warn!("assistbuild request compile error: {}", err);
                            });
                        };
             
                        log::info!("assistbuild request compile done. from: {:?}", stream.peer_addr().unwrap());
                        let _ = stream.flush();
                        let _ = stream.shutdown().await;
                        break;
                    }
                },
                Err(err) => {
                    log::warn!("buildassist connect data error: {}", err);
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