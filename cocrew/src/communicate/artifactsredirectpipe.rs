use std::{io, os::windows::io::AsRawHandle};
use prost::bytes;
use tokio::io::AsyncReadExt;

static WAITING_PIPE_INSTANCES: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8)
});

fn create_server(first_pipe_instance: bool) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    const PIPE_NAME: &str = r"\\.\pipe\compile_artifacts_sync";

    tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(first_pipe_instance)
        .access_inbound(true)
        .access_outbound(true)
        .in_buffer_size(65536 * 1024)
        .out_buffer_size(16)
        .max_instances(128)
        .create(PIPE_NAME)
}

async fn accept_connections(mut server: tokio::net::windows::named_pipe::NamedPipeServer) {
    loop {
        if let Err(error) = server.connect().await {
            log::error!("server failed to connect artifacts redirect named pipe: {:?}", error);
            return;
        }

        let mut connected = server;
        server = match create_server(false) {
            Ok(server) => server,
            Err(error) => {
                log::error!("server failed to replenish artifacts redirect named pipe: {:?}", error);
                return;
            }
        };

        tokio::spawn(async move {
            let pid = unsafe {
                let mut pid: u32 = 0;
                windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId(
                    connected.as_raw_handle() as _,
                    &mut pid as *mut u32,
                );
                pid
            };

            log::info!("connected to artifacts redirect named pipe server, client pid: {}", pid);

            if let Err(error) = handle(&mut connected).await {
                log::error!("server failed to handle artifacts redirect client {} {:?}", pid, error);
            }
        });
    }
}

pub fn compiler_redirect_artifacts() {
    let rt = { crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone() };

    let _runtime_guard = rt.enter();
    let mut waiting_servers = Vec::with_capacity(*WAITING_PIPE_INSTANCES);

    for index in 0..*WAITING_PIPE_INSTANCES {
        match create_server(index == 0) {
            Ok(server) => waiting_servers.push(server),
            Err(error) => {
                log::error!("failed to create artifacts redirect named pipe server instance {}/{}: {:?}", index + 1, *WAITING_PIPE_INSTANCES, error);
                break;
            }
        }
    }

    if waiting_servers.is_empty() {
        log::error!("failed to create any artifacts redirect named pipe server instances");
        return;
    }

    let count = waiting_servers.len();
    for server in waiting_servers {
        rt.spawn(accept_connections(server));
    }

    log::info!("artifacts redirect named pipe server ready with {} waiting instances", count);
}

async fn handle(pipe: &mut tokio::net::windows::named_pipe::NamedPipeServer) -> io::Result<()> {
    
    let mut prebuf = [0u8; 8];

    loop {
        let length = match pipe.read_exact(&mut prebuf).await {
            Ok(_) => {
                u64::from_le_bytes(prebuf)
            },
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset
                   || e.kind() == io::ErrorKind::BrokenPipe => break,
            Err(e) => return Err(e),
        };

        let mut buffer = vec![0u8; length as usize];
        let _ = match pipe.read_exact(&mut buffer).await {
            Ok(_) => {
                // 8       8                   8             8  
                //offset + clength + content + plen + path + continue
                let offset = i64::from_le_bytes(buffer[0..8].try_into().unwrap());
                let clen = u64::from_le_bytes(buffer[8..16].try_into().unwrap());
                
                let plen = u64::from_le_bytes(buffer[16 + clen as usize..(16 + clen as usize + 8)].try_into().unwrap()) as usize;
                let path = String::from_utf8_lossy(&buffer[16 + clen as usize + 8..(16 + clen as usize + 8 + plen)]);
                
                let iscontinue = u64::from_le_bytes(buffer[16 + clen as usize + 8 + plen..(16 + clen as usize + 8 + plen + 8)].try_into().unwrap());

                let ospath = std::ffi::OsString::from(&path.to_string());
                log::info!("received artifact file chunk: path: {}, offset: {}, clen: {}, iscontinue: {}", path, offset, clen, iscontinue);

                let context = buffer[16..(16 + clen as usize)].to_vec();
                let compiled_gen_result = if path.ends_with(".obj") {
                    crew::compiler::model::CompiledResult {
                        source_file: ospath.clone(),
                        obj: Some((ospath.clone(), offset, bytes::Bytes::from(context))),
                        pdb: None,
                        idb: None,
                    }
                }
                else if path.ends_with(".pdb") {
                    crew::compiler::model::CompiledResult {
                        source_file: ospath.clone(),
                        obj: None,
                        pdb: Some((ospath.clone(), offset, bytes::Bytes::from(context))),
                        idb: None,
                    }
                }
                else if path.ends_with(".idb") {
                    crew::compiler::model::CompiledResult {
                        source_file: ospath.clone(),
                        obj: None,
                        pdb: None,
                        idb: Some((ospath.clone(), offset, bytes::Bytes::from(context))),
                    }
                }
                else {
                    log::warn!("received unknown artifact file type: {}", path);
                    continue;
                };
    
                crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                    log::warn!("send artifact file to grpc channel failed: {:?}", err);
                });
                
                if iscontinue.eq(&0) {
                    //last chunk, send end
                    let compiled_gen_result = if path.ends_with(".obj") {
                        crew::compiler::model::CompiledResult {
                            source_file: ospath.clone(),
                            obj: Some((ospath, -1, bytes::Bytes::new())),
                            pdb: None,
                            idb: None,
                        }
                    }
                    else if path.ends_with(".pdb") {
                        crew::compiler::model::CompiledResult {
                            source_file: ospath.clone(),
                            obj: None,
                            pdb: Some((ospath, -1, bytes::Bytes::new())),
                            idb: None,
                        }
                    }
                    else if path.ends_with(".idb") {
                        crew::compiler::model::CompiledResult {
                            source_file: ospath.clone(),
                            obj: None,
                            pdb: None,
                            idb: Some((ospath, -1, bytes::Bytes::new())),
                        }
                    }   
                    else {
                        log::warn!("received unknown artifact file type: {}", path);
                        continue;
                    };
                     
                    crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                        log::warn!("send artifact end file chunk to grpc channel failed: {:?}", err);
                    });
                }
            },
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset
                   || e.kind() == io::ErrorKind::BrokenPipe => break,
            Err(e) => return Err(e),
        };

        //pipe.write_all(&buf[..n]).await?;
    }
    Ok(())
}