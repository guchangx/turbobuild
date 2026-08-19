
use crate::{artifactsredirect, log};

use tokio::{io::AsyncWriteExt, io::AsyncSeekExt, net::windows::named_pipe::{
    ClientOptions,
    NamedPipeClient,
}};

async fn connect() -> Option<NamedPipeClient>
{
    const PIPE_NAME:&str = r"\\.\pipe\compile_artifacts_sync";

    for i in 0..3 {
        match ClientOptions::new()
            .open(PIPE_NAME) {
            Ok(client) => {
                return Some(client);
            },
            Err(err) => {
                if err.raw_os_error() == Some(231) { //ERROR_PIPE_BUSY
                    unsafe {
                        let _ = windows_sys::Win32::System::Pipes::WaitNamedPipeW(PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>().as_ptr(), 300 * (i + 1));
                    }
                    continue;
                }
                else {
                    log!(debug, "redirect failed to connect artifacts named pipe: {:?}", err);
                    break;
                }
            }
        }
    }
    return None;
}

pub struct Artifacts {
    pub path: String,
    pub offset: i64,
    pub length: u64,
    pub content: Vec<u8>,
    pub done: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct Channel {
    pub tx: tokio::sync::mpsc::Sender<Artifacts>,
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<Artifacts>>>,
}

pub static REDIRECT_ARTIFACTS_CHANNEL: std::sync::LazyLock<Channel> = std::sync::LazyLock::new(|| {
    let (tx, rx) = tokio::sync::mpsc::channel::<Artifacts>(512);
    let channel = Channel { tx, rx: std::sync::Mutex::new(Some(rx)) };
    return channel;
});

pub fn send_artifact(artifact: Artifacts) -> Result<(), ()> {
    match REDIRECT_ARTIFACTS_CHANNEL.tx.try_send(artifact) {
        Ok(()) => Ok(()),
        Err(tokio::sync::mpsc::error::TrySendError::Full(artifact)) => {
            log!(warn, "artifact queue is full; waiting for pipe transfer capacity");
            REDIRECT_ARTIFACTS_CHANNEL
                .tx
                .blocking_send(artifact)
                .map_err(|error| {
                    log!(error, "artifact queue closed while waiting for capacity: {:?}", error);
                })
        }
        Err(tokio::sync::mpsc::error::TrySendError::Closed(artifact)) => {
            log!(error, "artifact queue is closed; dropping artifact for {}", artifact.path);
            Err(())
        }
    }
}

//TODO: every chunk should be 32KB 
fn redirect_artifacts_2_cocrew() {

    let mut rx =
        REDIRECT_ARTIFACTS_CHANNEL
        .rx
        .lock()
        .unwrap()
        .take().unwrap();

    let runtime = crate::REDIRECT_RUNTIME.handle().clone();
    let _ = runtime.spawn(async move {
        match connect().await {
            Some(mut client) => {
                log!(debug, "redirect success connect to artifacts named pipe");
                
                let mut chunks = Vec::<Artifacts>::new();

                while let Some(artifacts) = rx.recv().await {
                    log!(debug, "redirect send artifacts to named pipe: path: {}, offset: {}, length: {}", artifacts.path, artifacts.offset, artifacts.length);

                    if let Some(chunk) = chunks.iter_mut().find(|item| item.path == artifacts.path) {
                        if artifacts.length == 0 { // last chunk, send all
                            //offset + length + content + plen + path + continue
                            let tsize = 8 + 8 + chunk.length as usize + 8 + chunk.path.len() + 8;
                            let mut buffer = Vec::with_capacity(8 + tsize);

                            // write the length of the buffer first 8 bit
                            buffer.extend_from_slice(&tsize.to_le_bytes());

                            buffer.extend_from_slice(&(chunk.offset as i64).to_le_bytes());
                            buffer.extend_from_slice(&(chunk.length as u64).to_le_bytes());
                            buffer.extend_from_slice(chunk.content.as_slice());
                            buffer.extend_from_slice(&(chunk.path.len() as u64).to_le_bytes());
                            buffer.extend_from_slice(chunk.path.as_bytes());
                            buffer.extend_from_slice(&0u64.to_le_bytes());

                            if let Err(err) = client.write_all(&buffer).await {
                                log!(debug, "redirect failed to write artifacts to named pipe: {:?}", err);
                                break;
                            }
                            
                            client.flush().await.unwrap();
                            if let Some(done) = artifacts.done {
                                let _ = done.send(());
                                log!(trace, "redirect_artifacts_2_cocrew done path: {}", artifacts.path);
                            }
                            chunks.retain(|item| item.path != artifacts.path);
                            continue;
                        }
                        else if chunk.offset + chunk.length as i64 == artifacts.offset {
                            chunk.content.extend_from_slice(&artifacts.content);
                            chunk.length += artifacts.length;
                            if chunk.length >= 32768 /*32 * 1024*/ {
                                //offset + clength + content + plen + path + continue
                                let tsize = 8 + 8 + chunk.length as usize + 8 + chunk.path.len() + 8;
                                let mut buffer = Vec::with_capacity(8 + tsize);

                                // write the length of the buffer first 8 bit
                                buffer.extend_from_slice(&tsize.to_le_bytes());

                                buffer.extend_from_slice(&(chunk.offset as i64).to_le_bytes());
                                buffer.extend_from_slice(&(chunk.length as u64).to_le_bytes());
                                buffer.extend_from_slice(chunk.content.as_slice());
                                buffer.extend_from_slice(&(chunk.path.len() as u64).to_le_bytes());
                                buffer.extend_from_slice(chunk.path.as_bytes());
                                buffer.extend_from_slice(&1u64.to_le_bytes());

                                if let Err(err) = client.write_all(&buffer).await {
                                    log!(debug, "redirect failed to write artifacts to named pipe: {:?}", err);
                                    break;
                                }
                            }
                            chunks.retain(|item| item.path != artifacts.path);
                            continue;
                        }
                        else {
                            //offset + clength + content + plen + path + continue
                            let tsize = 8 + 8 + chunk.length as usize + 8 + chunk.path.len() + 8;
                            let mut buffer = Vec::with_capacity(8 + tsize);

                            // write the total length of the buffer first 8 bit
                            buffer.extend_from_slice(&tsize.to_le_bytes());

                            buffer.extend_from_slice(&(chunk.offset as i64).to_le_bytes());
                            buffer.extend_from_slice(&(chunk.length as u64).to_le_bytes());
                            buffer.extend_from_slice(chunk.content.as_slice());
                            buffer.extend_from_slice(&(chunk.path.len() as u64).to_le_bytes());
                            buffer.extend_from_slice(chunk.path.as_bytes());
                            buffer.extend_from_slice(&1u64.to_le_bytes());

                            if let Err(err) = client.write_all(&buffer).await {
                                log!(debug, "redirect failed to write artifacts to named pipe: {:?}", err);
                                break;
                            }
                            //repalace
                            chunk.offset = artifacts.offset;
                            chunk.length = artifacts.length;
                            chunk.content = artifacts.content;
                            continue;
                        }
                    }
                    else {
                        if let Some(done) = artifacts.done {
                            let _ = done.send(());
                            log!(trace, "redirect_artifacts_2_cocrew done path: {}", artifacts.path);
                        }
                        else{
                            chunks.push(artifacts);
                        }
                        continue;
                    }
                }
            }
            None => {
                log!(debug, "redirect failed to connect artifacts redirect named pipe");
            }
        }
    });
}

pub fn async_connect_artifacts_namedpipe() {
    redirect_artifacts_2_cocrew();
    //persist_artifact_to_local();
}

pub fn persist_artifact_to_local() {

    let mut rx =
        REDIRECT_ARTIFACTS_CHANNEL
        .rx
        .lock()
        .unwrap()
        .take().unwrap();

    let runtime = crate::REDIRECT_RUNTIME.handle().clone();

    let _ = runtime.spawn(async move {
        let mut openfiles = std::collections::HashMap::<String, tokio::fs::File>::new();

        while let Some(artifacts) = rx.recv().await {
            
            if !openfiles.contains_key(&artifacts.path) {
                let file = match tokio::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&artifacts.path.replace("obj", "art"))
                    .await
                {
                    Ok(file) => file,
                    Err(err) => {
                        log!(debug, "failed to open artifact file: {:?}, error: {:?}", artifacts.path, err);
                        continue;
                    }
                };

                log!(debug, "open artifact file: {:?}", artifacts.path);
                openfiles.insert(artifacts.path.clone(), file);
            }

            log!(debug, "write artifact file: {:?}, offset: {:?}, size: {:?}, content length: {:?}", artifacts.path, artifacts.offset, artifacts.length, &artifacts.content.len());

            if artifacts.length > 0 {
                let file = openfiles.get_mut(&artifacts.path).unwrap();
                file.seek(tokio::io::SeekFrom::Start(artifacts.offset as u64)).await.unwrap();
    
                if let Err(err) = file.write_all(&artifacts.content).await {
                    log!(debug, "failed to write artifacts to local file: {:?}, error: {:?}", artifacts.path, err);
                }
            } 
            else {
                let mut file = openfiles.remove(&artifacts.path).unwrap();
                if let Err(err) = file.flush().await {
                    log!(debug, "failed to flush artifacts to local file: {:?}, error: {:?}", artifacts.path, err);
                }
            }
        };
    });
}