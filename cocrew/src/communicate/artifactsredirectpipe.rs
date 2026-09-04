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
    pub struct Artifacts {
        pub offset: i64,
        pub length: u64,
        pub content: Vec<u8>,
    }

    let mut chunks = std::collections::HashMap::<String, Artifacts>::new();

    let mut buffer = Vec::with_capacity(1 * 1024 * 1024);
    loop {
        let length = match pipe.read_exact(&mut prebuf).await {
            Ok(_) => {
                u64::from_le_bytes(prebuf)
            },
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset
                   || e.kind() == io::ErrorKind::BrokenPipe => break,
            Err(e) => return Err(e),
        };

        buffer.clear();
        buffer.resize(length as usize, 0);
        let _ = match pipe.read_exact(&mut buffer).await {
            Ok(_) => {
                // 8       8                   8             8
                //offset + clength + content + plen + path + continue
                let offset = i64::from_le_bytes(buffer[0..8].try_into().unwrap());
                let clength = u64::from_le_bytes(buffer[8..16].try_into().unwrap());
                
                let plen = u64::from_le_bytes(buffer[16 + clength as usize..(16 + clength as usize + 8)].try_into().unwrap()) as usize;
                let path = String::from_utf8_lossy(&buffer[16 + clength as usize + 8..(16 + clength as usize + 8 + plen)]);
                
                let iscontinue = u64::from_le_bytes(buffer[16 + clength as usize + 8 + plen..(16 + clength as usize + 8 + plen + 8)].try_into().unwrap());

                let ospath = std::ffi::OsString::from(&path.to_string());
                log::info!("cocrew received artifact file chunk: path: {:?}, offset: {}, clen: {}, iscontinue: {}", ospath, offset,clength, iscontinue);
                if let Some(arti) = chunks.get_mut(&path.to_string()) {
                    if offset == 0 && clength == arti.offset as u64 {
                        arti.offset = 0;
                        arti.length += clength;

                        let prefix = &buffer[16..(16 + clength as usize)];
                        let prefixlen = prefix.len();
                        let allchunkslen = arti.content.len();
                        
                        arti.content.reserve(prefixlen + allchunkslen);
                        arti.content.resize(allchunkslen + prefixlen, 0);
                        arti.content.copy_within(0..allchunkslen, prefixlen);
                        arti.content[..prefixlen].copy_from_slice(prefix);
                    }
                    else if offset == 0 && clength != arti.offset as u64 {
                        let content = std::mem::take(&mut arti.content); //move ownership
                        let compiled_gen_result = generate_artifact(&path, arti.offset, bytes::Bytes::from(content));

                        crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                            log::warn!("send artifact file to grpc channel failed: {:?}", err);
                        });

                        //discontinuous data
                        arti.offset = offset;
                        arti.length = clength;
                        arti.content = buffer[16..(16 + clength as usize)].to_vec()
                    }
                    else if arti.offset + arti.length as i64 == offset {
                        arti.length += clength;
                        arti.content.extend_from_slice(&buffer[16..(16 + clength as usize)]);

                        if arti.length > 2097152 /*2 * 1024 * 1024*/ || iscontinue.eq(&0) {
                            let content = std::mem::take(&mut arti.content); //move ownership
                            let compiled_gen_result = generate_artifact(&path, arti.offset, bytes::Bytes::from(content));

                            crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                                log::warn!("send artifact file to grpc channel failed: {:?}", err);
                            });

                            arti.offset = 0;
                            arti.length = 0;
                        }

                        if iscontinue.eq(&0) {
                            //last chunk, send end
                            let compiled_gen_result = generate_artifact(&path, -1, bytes::Bytes::new());
                            crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                                log::warn!("send artifact end file chunk to grpc channel failed: {:?}", err);
                            });
                        }
                    }
                    else {
                        let content = std::mem::take(&mut arti.content); //move ownership
                        let compiled_gen_result = generate_artifact(&path, arti.offset, bytes::Bytes::from(content));

                        crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                            log::warn!("send artifact file to grpc channel failed: {:?}", err);
                        });

                        //discontinuous data
                        arti.offset = offset;
                        arti.length = clength;
                        arti.content = buffer[16..(16 + clength as usize)].to_vec()
                    }

                    if 0 == arti.offset && 0 == arti.length {
                        chunks.remove(&path.to_string());
                    }
                }
                else {
                    if iscontinue.eq(&0) {
                        let compiled_gen_result = generate_artifact(&path, offset, bytes::Bytes::from(buffer[16..(16 + clength as usize)].to_vec()));
                        crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                            log::warn!("send artifact file to grpc channel failed: {:?}", err);
                        });

                        let compiled_gen_result = generate_artifact(&path, -1, bytes::Bytes::new());
                        crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                            log::warn!("send artifact end file chunk to grpc channel failed: {:?}", err);
                        });
                    }
                    else {
                        let arti = Artifacts {
                            offset: offset,
                            length: clength,
                            content: buffer[16..(16 + clength as usize)].to_vec(),
                        };
                        chunks.insert(path.to_string(), arti);
                    }
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

fn generate_artifact(path: &std::borrow::Cow<str>, offset: i64, context: bytes::Bytes) -> crew::compiler::model::CompiledResult {
    let ospath = std::ffi::OsString::from(path.to_string());
    
    let compiled_gen_result = if path.ends_with(".obj") {
        crew::compiler::model::CompiledResult {
            source_file: ospath.clone(),
            obj: Some((ospath, offset, context)),
            pdb: None,
            idb: None,
        }
    }
    else if path.ends_with(".pdb") {
        crew::compiler::model::CompiledResult {
            source_file: ospath.clone(),
            obj: None,
            pdb: Some((ospath, offset, context)),
            idb: None,
        }
    }
    else if path.ends_with(".idb") {
        crew::compiler::model::CompiledResult {
            source_file: ospath.clone(),
            obj: None,
            pdb: None,
            idb: Some((ospath, offset, context)),
        }
    }
    else {
        crew::compiler::model::CompiledResult {
            source_file: ospath.clone(),
            obj: None,
            pdb: None,
            idb: None
        }
    };
    return compiled_gen_result;
}