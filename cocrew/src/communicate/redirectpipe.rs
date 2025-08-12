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
                    let server_ = server_.clone();
                    let tx_ = tx_.clone();
                    let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {
                        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel::<MirrorCommand>();
                    
                        let mut data = vec![0; 1024];
                        let mut server_ = server_.lock().await;
                        match server_.read(&mut data).await {
                            Ok(size) => {
                                if size > 0 as usize {
                                    let message = String::from_utf8_lossy(&data[..size]);
                                    log::info!("received mirror command: {}", message);
                                    if let Ok(mirror_cmd) = crate::serde_json::from_str::<MirrorCommand>(&message)
                                        .map_err(|e| {
                                            log::error!("failed to parse mirror command: {}", e);
                                        }) {
                                        if let Some(tx) = tx_.as_ref() {
                                            log::trace!("send mirror command to grpc: {:?}", mirror_cmd);
                                            let sender = tx.lock().await;
                                            sender.send((mirror_cmd, oneshot_tx)).await.unwrap();
                                            drop(sender);
                                        }
                                    }
                                }
                                else {
                                    log::warn!("no data received from pipe, disconnecting.");
                                    let _ = server_.disconnect();
                                    return;
                                }
                            },
                            Err(e) => {
                                log::error!("failed to read from pipe: {}", e);
                                return;
                            }
                        }

                        match tokio::time::timeout(tokio::time::Duration::from_secs(15), oneshot_rx).await {
                            Ok(Ok(rx)) => {
                                let response = serde_json::to_string(&rx).map_err(|e| {
                                    log::error!("failed to serialize response: {}", e);
                                }).unwrap();
                                log::info!("received mirror command response: {}", response);
                                match server_.write(response.as_bytes()).await {
                                    Ok(_) => {
                                        log::info!("sent mirror command response: {}", response);
                                    },
                                    Err(e) => {
                                        log::error!("failed to write mirror command response to pipe: {}", e);
                                    }
                                }
                            },
                            Ok(Err(e)) => {
                                log::error!("failed to receive mirror command response from oneshot: {}", e);
                            },
                            Err(e) => {
                                log::error!("failed to receive mirror command response timeout: {}", e);
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

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn compiler_redirect_request_test() {
        println!("test compiler_redirect_request");
        tools::logger::init_once_logger();
        let (namedpipe_tx, mut namedpipe_rx) = tokio::sync::mpsc::channel(128);
        compiler_redirect_request(std::sync::Arc::new(Some(tokio::sync::Mutex::new(namedpipe_tx))));

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let client = connect().await.unwrap();
        let (mut reader, mut writer) = tokio::io::split(client);

        //write meessage to pipe client
        for i in 0..10 {
            let command = MirrorCommand {
                id: i,
                command: "test".to_string(),
                args: std::collections::HashMap::new(),
            };

            let message = format!("{{\"id\": {}, \"command\": \"{}\", \"args\": {{{}}}}}",
                    command.id,
                    command.command,
                    command.args.iter()
                        .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
                        .collect::<Vec<_>>()
                        .join(", "));

            writer.write_all(message.as_bytes()).await.unwrap();
        }

        //read message from mspc
        for _ in 0..10 {
            let message = namedpipe_rx.recv().await;
            let (command, callback) = message.unwrap();
            println!("received command in test: {:?}", command);
            let response = MirrorCommand {
                id: command.id,
                command: "response".to_string(),
                args: std::collections::HashMap::new(),
            };
            callback.send(response).unwrap();
        }

        //read message from pipe client
        let mut data = vec![0; 1024];
        for _ in 0..10 {
            match reader.read(&mut data).await {
                Ok(size) if size > 0 => {
                    let message = String::from_utf8_lossy(&data[..size]);
                    println!("received message in test: {}", message);
                },
                Ok(_) => {
                    println!("no data received");
                },
                Err(e) => {
                    println!("failed to read from pipe: {}", e);
                }
            }
        }

        println!("test compiler_redirect_request done.");
    }

    #[test]
    fn test_parse_mirror_command() {
        let mut args = std::collections::HashMap::new();
        args.insert("key".to_string(), r#"\\?\D:\turbobuild\target\debug\Replica"#.to_string());

        let command = MirrorCommand {
            id: 0,
            command: "NtQueryDirectoryFile".to_string(),
            args: args,
        };

        let json = format!("{{\"id\": {}, \"command\": \"{}\", \"args\": {{{}}}}}",
                    command.id,
                    command.command,
                    command.args.iter()
                        .map(|(k, v)| format!("\"{}\": {:?}", k, v))
                        .collect::<Vec<_>>()
                        .join(", "));

        println!("command: {}", json);


        let result = crate::serde_json::from_str::<MirrorCommand>(&json);
        assert!(result.is_ok());
    }
}