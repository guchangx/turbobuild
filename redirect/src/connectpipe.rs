use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use tokio::task::Id;

async fn connect() -> std::result::Result<tokio::net::windows::named_pipe::NamedPipeClient, std::io::Error> {
    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";
    let client = loop {
        match tokio::net::windows::named_pipe::ClientOptions::new().open(PIPE_NAME) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_PIPE_BUSY as i32) => (),
            Err(e) => return Err(e),
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    };

    return Ok(client);
}

struct SyncCommand {
    id: u32,
    command: String,
    args: std::collections::HashMap<String, String>,
    responder: tokio::sync::oneshot::Sender<String>,
}
struct Channel {
    tx: tokio::sync::mpsc::Sender<SyncCommand>,
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<SyncCommand>>>,
}

static NET_REDIRECT_CHANNEL: std::sync::LazyLock<Channel> = std::sync::LazyLock::new(|| {
        let (tx, rx) = tokio::sync::mpsc::channel::<SyncCommand>(512);
        let channel = Channel { tx, rx: std::sync::Mutex::new(Some(rx)) };
        return channel;
});

async fn connect_named_pipe() {

    let client = connect().await.unwrap();
    let (mut reader, mut writer) = tokio::io::split(client);

    type Map = std::collections::HashMap<u32, tokio::sync::oneshot::Sender<String>>;
    let responders: std::sync::Arc<std::sync::Mutex<Map>> = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let responders_ = responders.clone();
    let _ = crate::RUNTIME.lock().unwrap().spawn(async move {
        let mut data = vec![0; 1024];
        loop {
            match reader.read(&mut data).await {
                Ok(size) if size > 0 => {
                    let message = String::from_utf8_lossy(&data[..size]);
                    crate::log!(info, "received message: {}", message);

                    if let Some(responder) = responders.lock().unwrap().remove(&0u32) {
                        let _ = responder.send(message.to_string());
                    }
                }
                Ok(_) => break,
                Err(e) => {
                    let err = format!("failed to read from pipe: {}", e);
                    crate::log!(error, "{}", err);
                    break;
                }
            }
        }
    });
    
    let _ = crate::RUNTIME.lock().unwrap().spawn(async move {
        let mut rx = NET_REDIRECT_CHANNEL.rx.lock().unwrap().take().unwrap();
        while let Some(synccmd) = rx.recv().await {
            let command = synccmd.command;

            match writer.write(command.as_bytes()).await {
                Ok(size) => {
                    responders_.lock().unwrap().insert(synccmd.id, synccmd.responder);
                },
                Err(e) => {
                    let err = format!("failed to write to pipe: {}", e);
                    crate::log!(error, "{}", err);
                }
            }
        }
    });

}