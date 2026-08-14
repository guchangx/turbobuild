
use std::any::Any;

use crate::log;

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
                    tokio::time::sleep(std::time::Duration::from_millis(300 * (i + 1))).await;
                    continue;
                }
                else {
                    log!(debug, "failed to connect artifacts redirect named pipe: {:?}", err);
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
                log!(debug, "success connect to artifacts redirect named pipe");
                while let Some(artifacts) = rx.recv().await {

                    let mut buffer = Vec::with_capacity(
                        //offset + length + content + path
                        8 + 8 + artifacts.content.len() + artifacts.path.len()
                    );

                    // write the length of the buffer first 8 bit
                    if let Err(err) = client.write_all(&buffer.capacity().to_le_bytes()).await {
                        log!(debug, "failed to write artifacts length to named pipe: {:?}", err);
                        break;
                    }

                    buffer.extend_from_slice(&(artifacts.offset as i64).to_le_bytes());
                    buffer.extend_from_slice(&(artifacts.length as u64).to_le_bytes());
                    buffer.extend_from_slice(artifacts.content.as_slice());
                    buffer.extend_from_slice(artifacts.path.as_bytes());

                    if let Err(err) = client.write_all(&buffer).await {
                        log!(debug, "failed to write artifacts to named pipe: {:?}", err);
                        break;
                    }

                    log!(trace, "redirect_artifacts_2_cocrew path: {} offset: {} content length: {}", artifacts.path, artifacts.offset, artifacts.length);

                    if artifacts.length == 0 {
                        client.flush().await.unwrap();
                        if let Some(done) = artifacts.done {
                            let _ = done.send(());
                            log!(trace, "redirect_artifacts_2_cocrew done path: {}", artifacts.path);
                        }
                    }
                }
            }
            None => {
                log!(debug, "failed to connect artifacts redirect named pipe");
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