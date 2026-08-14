use std::{io, os::windows::io::AsRawHandle};
use tokio::io::AsyncReadExt;

pub fn compiler_redirect_artifacts() {

    const PIPE_NAME:&str = r"\\.\pipe\compile_artifacts_sync";
    let rt = { crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone() };
    rt.spawn(async move {
        let mut server = tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(true)
        .access_inbound(true)
        .access_outbound(true)
        .in_buffer_size(65536)  
        .out_buffer_size(16)
        .max_instances(64)
        .create(PIPE_NAME).map_err(|e| {
            log::error!("failed to create artifacts redirect named pipe, err: {:?}", e);
            e
        }).unwrap();

        loop {
            server.connect().await.map_err(|e| {
                log::error!("failed to connect artifacts redirect named pipe, err: {:?}", e);
                e
            }).unwrap();

            let mut connected = server;
            server = tokio::net::windows::named_pipe::ServerOptions::new()
                .access_inbound(true)
                .access_outbound(true)
                .in_buffer_size(65536)  
                .out_buffer_size(16)
                .max_instances(64)
                .create(PIPE_NAME).map_err(|e| {
                    log::error!("failed to create artifacts redirect named pipe, err: {:?}", e);
                    e
                }).unwrap();

            tokio::spawn(async move {
                unsafe {
                    let mut pid: u32 = 0;
                    windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId(connected.as_raw_handle() as _, &mut pid as *mut u32);
                    log::info!("connected to artifacts redirect named pipe, client pid: {}", pid);
                }

                if let Err(e) = handle(&mut connected).await {
                    if e.kind() == io::ErrorKind::UnexpectedEof {

                    }
                    else {
                        log::error!("failed to handle client: {:?}", e);
                    }
                }
            });
        }
    });
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
                //offset + length + content + path 
                let offset = i64::from_le_bytes(buffer[0..8].try_into().unwrap());
                let len = u64::from_le_bytes(buffer[8..16].try_into().unwrap());
                let context = buffer[16..(16 + len as usize)].to_vec();
                let path = String::from_utf8_lossy(&buffer[(16 + len as usize)..]);
                
                let ospath = std::ffi::OsString::from(&path.to_string());

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
                    log::warn!("received unknown artifact file type: {}", path);
                    continue;
                };
 
                crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                    log::warn!("send unready .obj file path to channel failed: {:?}", err);
                });
            },
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset
                   || e.kind() == io::ErrorKind::BrokenPipe => break,
            Err(e) => return Err(e),
        };

        //pipe.write_all(&buf[..n]).await?;
    }
    Ok(())
}