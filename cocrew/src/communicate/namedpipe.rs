use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

pub fn compiler_redirect_request(tx: std::sync::Arc<Option<tokio::sync::Mutex<tokio::sync::mpsc::Sender<String>>>>) {
    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";

    let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {

        loop {
            let mut server = tokio::net::windows::named_pipe::ServerOptions::new()
                .first_pipe_instance(true)
                .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
                .access_inbound(true)
                .access_outbound(true)
                .max_instances(1024)
                .create(PIPE_NAME).unwrap();

            server.connect().await.unwrap();
            let tx_ = tx.clone();
            let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {

                loop {

                    let ready = server.ready(tokio::io::Interest::READABLE | tokio::io::Interest::WRITABLE).await.unwrap();
                    
                    if ready.is_readable() {
                        let mut data = vec![0; 1024];
                        match server.read(&mut data).await {
                            Ok(size) => {
                                if size > 0 {
                                    let message = String::from_utf8_lossy(&data[..size]);
                                    if let Some(tx) = tx_.as_ref() {
                                        let sender = tx.lock().await;
                                        sender.send("pipe connected".to_string()).await.unwrap();
                                    }
                        
                                    log::info!("received message: {}", message);
                                }
                            },
                            Err(e) => {
                                log::error!("failed to read from pipe: {}", e);
                            }
                        }
                    }

                    if ready.is_writable() {
                        let message = "Hello from server";
                        match server.write(message.as_bytes()).await {
                            Ok(_) => {
                                log::info!("sent message: {}", message);
                            },
                            Err(e) => {
                                log::error!("failed to write to pipe: {}", e);
                            }
                        }
                    }
                }
            });
        }
    });
}