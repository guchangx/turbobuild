use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct MirrorCommand {
    pub id: u32,
    pub command: String,
    pub args: std::collections::HashMap<String, String>,
}

type Responders = tokio::sync::oneshot::Sender<MirrorCommand>;
pub fn compiler_redirect_request(tx: std::sync::Arc<Option<tokio::sync::Mutex<tokio::sync::mpsc::Sender<(MirrorCommand, Responders)>>>>) {

    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";

    let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {
        let mut server = tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(true)
            .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
            .access_inbound(true)
            .access_outbound(true)
            .in_buffer_size(512)
            .out_buffer_size(512)
            .create(PIPE_NAME).unwrap();

        loop {

            server.connect().await.unwrap();
            let tx_ = tx.clone();
            let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {
                let server_ = std::sync::Arc::new(tokio::sync::Mutex::new(server));
                loop {
                    let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel::<MirrorCommand>();

                    let server_ = server_.clone();
                    let ready = server_.lock().await.ready(tokio::io::Interest::READABLE | tokio::io::Interest::WRITABLE).await.unwrap();

                    if ready.is_readable() {
                        let mut data = vec![0; 1024];
                        match server_.lock().await.read(&mut data).await {
                            Ok(size) => {
                                if size > 0 {
                                    let message = String::from_utf8_lossy(&data[..size]);
                                    if let Some(mirror_cmd) = crate::serde_json::from_str::<MirrorCommand>(&message)
                                        .map_err(|e| {
                                            log::error!("failed to parse message: {}", e);
                                        })
                                        .ok() {
                                            if let Some(tx) = tx_.as_ref() {
                                                let sender = tx.lock().await;
                                                sender.send((mirror_cmd, oneshot_tx)).await.unwrap();
                                            }
                                        }
                                    log::info!("received message: {}", message);
                                }
                            },
                            Err(e) => {
                                log::error!("failed to read from pipe: {}", e);
                            }
                        }
                    }
                    
                    let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {

                        let command_result = oneshot_rx.await.unwrap();

                        let response = serde_json::to_string(&command_result).map_err(|e| {
                            log::error!("failed to serialize response: {}", e);
                        }).unwrap();

                        if ready.is_writable() {
                            match server_.lock().await.write(response.as_bytes()).await {
                                Ok(_) => {
                                    log::info!("sent message: {}", response);
                                },
                                Err(e) => {
                                    log::error!("failed to write to pipe: {}", e);
                                }
                            }
                        }
                    });
                }
            });

            server = tokio::net::windows::named_pipe::ServerOptions::new()
                .first_pipe_instance(false)
                .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
                .access_inbound(true)
                .access_outbound(true)
                .in_buffer_size(512)
                .out_buffer_size(512)
                .create(PIPE_NAME).unwrap();
        }
    });
}