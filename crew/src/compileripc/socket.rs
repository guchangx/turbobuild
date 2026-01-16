
use std::io::BufRead;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct SystemMarkDrop {
    handle: tools::ptr::HandleBox, 
}

impl Drop for SystemMarkDrop {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(*self.handle.get());
        }
    }
}

static SYSTEM_MARK_DROP_GUARD: std::sync::OnceLock<SystemMarkDrop> = std::sync::OnceLock::new();

fn set_system_mark() {

    log::debug!("set crewruning system mark.");
    let data = b"run\0";

    let name = "Local\\CrewRunningSystemMark";
    let name_utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let hmap = windows_sys::Win32::System::Memory::CreateFileMappingW(
            windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
            std::ptr::null_mut(),
            windows_sys::Win32::System::Memory::PAGE_READWRITE,
            0,
            data.len() as u32,
            name_utf16.as_ptr(),
        );

        if hmap.is_null() {
            log::warn!("CreateFileMappingW failed, mark not set. GetLastError={}", windows_sys::Win32::Foundation::GetLastError());
            return;
        }

        let mark = SystemMarkDrop {
            handle: tools::ptr::HandleBox::new(hmap),
        };
        let _ = SYSTEM_MARK_DROP_GUARD.set(mark);
        
        let view = windows_sys::Win32::System::Memory::MapViewOfFile(
            hmap,
            windows_sys::Win32::System::Memory::FILE_MAP_WRITE,
            0,
            0,
            data.len() as usize,
        );

        if view.Value.is_null() {
            log::warn!("MapViewOfFile failed, mark not set. GetLastError={}", windows_sys::Win32::Foundation::GetLastError());
            return;
        }

        let pview = view.Value as *mut u8;
        
        std::ptr::copy_nonoverlapping(data.as_ptr(), pview, data.len());

        let _ = windows_sys::Win32::System::Memory::FlushViewOfFile(pview as *const std::ffi::c_void, data.len() as usize);
        windows_sys::Win32::System::Memory::UnmapViewOfFile(view);
    }
}

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

        set_system_mark();
        
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let metrics = runtime.metrics(); 
            log::debug!("new buildassist connection socket. runtime: {:#?} {:#?}", metrics.num_workers(), metrics.num_alive_tasks());
            
            let distributor = self.distributor.clone();
            let runtime_ = runtime.clone();
            let env = self.work_env.clone();

            let _ =  runtime.spawn(async { Self::handle_buildassist_stream(stream, runtime_, env, distributor).await;});
        }
    }
    
    async fn handle_buildassist_stream(mut stream: tokio::net::TcpStream, runtime: std::sync::Arc<tokio::runtime::Handle>, work_env: crate::platform::windows::WindowsCompilerEnv, distor: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>) {
       
        let mut data = String::new();
        let mut buffer = [0 as u8; 1024];
        stream.set_nodelay(true).unwrap();
        
        loop {
            match stream.read(&mut buffer).await {
                Ok(size) => {
                    data.push_str(std::str::from_utf8(&buffer[..size]).unwrap());

                    if size < buffer.len() || ( size == buffer.len() && buffer.ends_with(b"}}")) {
                        log::debug!("buildassist connection data: {}", data);

                        let input: serde_json::Value = serde_json::from_str(data.as_str()).expect(&format!("invalid json data: {}", data));

                        let solution = input["solution"].as_str().unwrap();
                        let project = input["project"].as_str().unwrap();
                        let compiler = input["compiler"].as_str().unwrap();
                        let working = input["working_dir"].as_str().unwrap();
                        let commands = input["commands"].as_array().unwrap();
                        let r#type = input["type"].as_str().unwrap();
                        let envs = input["envs"].as_object().unwrap()
                            .iter()
                            .map(|(k, v)| (std::ffi::OsString::from(k), std::ffi::OsString::from(v.as_str().unwrap())))
                            .collect::<std::collections::HashMap<_, _>>();
                        
                        let input = crate::compiler::model::CompilerInput {
                            solution: std::ffi::OsString::from(solution),
                            project: std::ffi::OsString::from(project),
                            compiler_path: std::ffi::OsString::from(compiler),
                            compiler_working_dir: std::ffi::OsString::from(working),
                            compiler_commands: commands.iter().map(|item| item.as_str().unwrap().into()).collect(),
                            build_and_compiler_type: std::ffi::OsString::from(r#type),
                            envs: envs
                        };

                        let stream = std::sync::Arc::new(tokio::sync::Mutex::new(stream));
                        let stream_ = stream.clone();

                        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
                        let _ = runtime.spawn(async move {
                            
                            let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(3 * 60));
                            timer.tick().await;
 
                            let mut stop_rx = std::pin::Pin::new(&mut stop_rx);
       
                            for _ in 0..4 {
                                tokio::select! {
                                    _ = timer.tick() => {
                                        let _ = stream_.lock().await.write_all(b"waiting...").await.unwrap_or_else(|err| {
                                            log::warn!("assistbuild request compile output error: {}", err);
                                        });
                                        let _ = stream_.lock().await.flush().await;
                                    },
                                    _ = &mut stop_rx => {
                                        break;
                                    }
                                }
                            }
                        });
                        
                        let stream_ = stream.clone();

                        let output_callback = move |result: crate::compiler::model::CompilerOutput| {
                            let stream_ = stream_.clone();
                            async move {
                                if !result.out.is_empty() {
                                    for line in result.out.lines() {
                                        let mut line = line.unwrap();
                                        line.push_str("\r\n");
                                        let _ = stream_.lock().await.write_all(line.as_bytes()).await.unwrap_or_else(|err| {
                                            log::warn!("assistbuild request compile output error: {}", err);
                                        });
                                        let _ = stream_.lock().await.flush().await;
                                    };
                                }

                                if !result.err.is_empty() {
                                    for line in result.err.lines() {
                                        let mut line = line.unwrap();
                                        line.push_str("\r\n");
                                        let _ = stream_.lock().await.write_all(line.as_bytes()).await.unwrap_or_else(|err| {
                                            log::warn!("assistbuild request compile error: {}", err);
                                        });
                                        let _ = stream_.lock().await.flush().await;
                                    };
                                }
                            }
                        };

                        let result = crate::compiler::interface::request_compile(input, runtime.clone(), if work_env.winkits_includes_path.is_empty() {Some(work_env)} else { None }, distor, output_callback).await;  
                        
                        if !stop_tx.is_closed() {
                            stop_tx.send(()).unwrap();
                        }

                        if !result.out.is_empty() {
                            for line in result.out.lines() {
                                let mut line = line.unwrap();
                                line.push_str("\r\n");
                                let _ = stream.lock().await.write_all(line.as_bytes()).await.unwrap_or_else(|err| {
                                    log::warn!("assistbuild request compile output error: {}", err);
                                });
                                let _ = stream.lock().await.flush().await;
                            };
                        }

                        if !result.err.is_empty() {
                            for line in result.err.lines() {
                                let mut line = line.unwrap();
                                line.push_str("\r\n");
                                let _ = stream.lock().await.write_all(line.as_bytes()).await.unwrap_or_else(|err| {
                                    log::warn!("assistbuild request compile error: {}", err);
                                });
                                let _ = stream.lock().await.flush().await;
                            };
                        }

                        log::info!("assistbuild request compile {} done. from: {:?}", project, stream.lock().await.peer_addr().unwrap());

                        let _ = stream.lock().await.shutdown().await;
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