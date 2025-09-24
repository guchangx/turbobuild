
use std::os::windows::io::AsHandle;
use std::os::windows::io::AsRawHandle;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct MirrorSysCall {
    pub id: u32,
    pub api: String,
    pub args: std::collections::HashMap<String, String>,
}

pub struct CHANNEL {
    pub grpc_to_namedpipe_tx: std::sync::Arc<tokio::sync::broadcast::Sender<MirrorSysCall>>,
    pub grpc_to_namedpipe_rx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::broadcast::Receiver<MirrorSysCall>>>,
}

pub static GRPC_TO_NAMEDPIPE_CHANNEL: std::sync::LazyLock<CHANNEL> = std::sync::LazyLock::new(|| {

    let (tx, rx) = tokio::sync::broadcast::channel(128);
    let grpc_to_namedpipe_tx =  std::sync::Arc::new(tx);
    let grpc_to_namedpipe_rx = std::sync::Arc::new(tokio::sync::Mutex::new(rx));

    return CHANNEL {
        grpc_to_namedpipe_tx,
        grpc_to_namedpipe_rx
    };
});

pub fn compiler_redirect_syscall() {

    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";

    let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};
    let rt_ = rt.clone();
    let _ = rt.spawn(async move {
        let mut counter = 0;
        let rt_ = rt_.clone();
        let makeserver = |first| { tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(first)
            .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
            .access_inbound(true)
            .access_outbound(true)
            .in_buffer_size(65536)  
            .out_buffer_size(65536)
            //.write_dac(true)
            //.write_owner(true)
            //.access_system_security(true)
            .reject_remote_clients(false)
            .create(PIPE_NAME).unwrap()
        };

        let mut server = makeserver(true);

        loop {
            log::info!("namedpipe connecting counter: {:?}", counter);
            server.connect().await.expect("server failed to connect to named pipe");

            let handle = server.as_handle();
            let mut pid: u32 = 0;
            unsafe {
                windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId(handle.as_raw_handle(), &mut pid as *mut u32);
            }
            
            let (mut reader, mut writer) = tokio::io::split(server);
            
            server = makeserver(false);

            log::info!("namedpipe connected counter: {:?} success, client pid: {:?}", counter, pid);
            
            //return the syscall response to caller in func.rs
            rt_.spawn(async move {
                let mut rx = { GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_rx.lock().await.resubscribe() };
 
                loop {
                    if let Ok(response) = rx.recv().await {
                        if response.id / 10000 == pid {
                            let response = format_mirror_command(&response);
                            match writer.write(response.as_bytes()).await {
                                Ok(n) => {
                                    log::info!("success sent mirror command response: {}", n);
                                    writer.flush().await.unwrap();
                                },
                                Err(err) => {
                                    log::error!("failed to write mirror command response to pipe: {}", err);
                                    continue;
                                }
                            }
                        }
                    }
                    else {
                        log::warn!("mirror command response channel closed, dropped receiver.");
                        break;
                    }
                }
                writer.shutdown().await.unwrap();
                drop(rx);
            });

            let _ = rt_.spawn(async move {
                log::trace!("pipe connected count {} success", counter);
                loop {
                    //read command form namedpipe and send it to grpc.
                    let mut data = vec![0; 1024];
                    match reader.read(&mut data).await {
                        Ok(size) => {
                            if size > 0 as usize {
                                let message = String::from_utf8_lossy(&data[..size]);
                                log::info!("received mirror syscall: {}", message);
                                if let Ok(mirror_cmd) = crate::serde_json::from_str::<MirrorSysCall>(&message)
                                    .map_err(|e| {
                                        log::error!("failed to parse mirror syscall: {}", e);
                                    }) {

                                    if let Some(sender) = crate::communicate::unpackager::NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_tx.as_ref() {
                                        log::trace!("send mirror syscall to grpc: {:?}", mirror_cmd);
                                        sender.send(mirror_cmd).await.unwrap();
                                    }
                                }
                            }
                            else {
                                log::warn!("no data received from pipe, disconnecting.");
                                return;
                            }
                        },
                        Err(e) => {
                            log::error!("failed to read from pipe: {}", e);
                            return;
                        }
                    }
                }
            });
            counter += 1;
        }
    });
}

pub fn format_mirror_command(call: &MirrorSysCall) -> String {
    let response = format!("{{\"id\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
        call.id,
        call.api,
        call.args.iter()
            .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(", "));
    return response;
}

#[cfg(test)]
mod tests {

    use super::*;
    async fn connect() -> std::result::Result<tokio::net::windows::named_pipe::NamedPipeClient, std::io::Error> {
        const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";
        let client = loop {
            match tokio::net::windows::named_pipe::ClientOptions::new()
            .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
            .open(PIPE_NAME) {
                Ok(client) => break client,
                Err(e) if e.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_PIPE_BUSY as i32) => (),
                Err(e) => return Err(e),
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        };

        return Ok(client);
    }

    #[tokio::test]
    async fn compiler_redirect_syscall_test() {
        println!("test compiler_redirect_syscall");
        tools::logger::init_once_logger();

        let namedpipe_tx = crate::communicate::unpackager::NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_tx.clone();
        let mut namedpipe_rx = crate::communicate::unpackager::NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_rx.lock().await;

        compiler_redirect_syscall();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let client = connect().await.unwrap();
        let client = connect().await.unwrap();
        let client = connect().await.unwrap();
        let client = connect().await.unwrap();
        let client = connect().await.unwrap();
        let client = connect().await.unwrap();

        redirectdll::syscallredirect::async_connect_syscall_namedpipe();
        redirectdll::syscallredirect::async_connect_syscall_namedpipe();
        redirectdll::syscallredirect::async_connect_syscall_namedpipe();
        redirectdll::syscallredirect::async_connect_syscall_namedpipe();
        redirectdll::syscallredirect::async_connect_syscall_namedpipe();
        redirectdll::syscallredirect::async_connect_syscall_namedpipe();

        let (mut reader, mut writer) = tokio::io::split(client);

        //write meessage to pipe client
        for i in 0..10 {
            let command = MirrorSysCall {
                id: i,
                api: "test".to_string(),
                args: std::collections::HashMap::new(),
            };

            let message = format!("{{\"id\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
                    command.id,
                    command.api,
                    command.args.iter()
                        .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
                        .collect::<Vec<_>>()
                        .join(", "));

            writer.write_all(message.as_bytes()).await.unwrap();
        }
        writer.shutdown().await.unwrap();

        //read message from mspc
        if let Some(mut rx) = namedpipe_rx.take() {
            for _ in 0..10 {
                let message = rx.recv().await;
                let command = message.unwrap();
                println!("received command in test: {:?}", command);
                let response = MirrorSysCall {
                    id: command.id,
                    api: "response".to_string(),
                    args: std::collections::HashMap::new(),
                };
            }
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

        let command = MirrorSysCall {
            id: 0,
            api: "NtQueryDirectoryFile".to_string(),
            args: args,
        };

        let json = format!("{{\"id\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
                    command.id,
                    command.api,
                    command.args.iter()
                        .map(|(k, v)| format!("\"{}\": {:?}", k, v))
                        .collect::<Vec<_>>()
                        .join(", "));

        println!("command: {}", json);


        let result = crate::serde_json::from_str::<MirrorSysCall>(&json);
        assert!(result.is_ok());
    }
}