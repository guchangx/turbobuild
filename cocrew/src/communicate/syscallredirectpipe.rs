
use std::os::windows::io::AsHandle;
use std::os::windows::io::AsRawHandle;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct MirrorSysCall {
    pub id: u32,
    pub command: String,
    pub args: std::collections::HashMap<String, String>,
}

type Responders = tokio::sync::oneshot::Sender<MirrorSysCall>;

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
        let mut server = tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(true)
            .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
            .access_inbound(true)
            .access_outbound(true)
            .in_buffer_size(65536)  
            .out_buffer_size(65536)
            //.write_dac(true)
            //.write_owner(true)
            //.access_system_security(true)
            .reject_remote_clients(false)
            .create(PIPE_NAME).unwrap();

        loop {
            server.connect().await.unwrap();
            let now = std::time::Instant::now();
            
            let handle = server.as_handle();
            let mut pid: u32 = 0;
            unsafe {
                windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId(handle.as_raw_handle(), &mut pid as *mut u32);
            }
            log::info!("namedpipe connected count: {:?} success, client pid: {:?}", counter, pid);

            let (mut reader, mut writer) = tokio::io::split(server);

            server = tokio::net::windows::named_pipe::ServerOptions::new()
                .first_pipe_instance(false)
                .pipe_mode(tokio::net::windows::named_pipe::PipeMode::Message)
                .access_inbound(true)
                .access_outbound(true)
                .in_buffer_size(65536)  
                .out_buffer_size(65536)
                //.write_dac(true)
                //.write_owner(true)
                //.access_system_security(true)
                .reject_remote_clients(false)
                .create(PIPE_NAME).unwrap();
            
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

                    /* 
                    loop {
                        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel::<MirrorCommand>();
                        if let Some(tx) = crate::communicate::unpackager::NAMEDPIPE_AND_GRPC_CHANNEL.namedpipe_and_grpc_tx.as_ref() {
                            
                            let mirror_cmd = MirrorCommand {
                                id: 111111111,
                                command: "".to_string(),
                                args: std::collections::HashMap::new(),
                            };

                            log::trace!("send mirror command to grpc: {:?}", mirror_cmd);
                            let sender = tx.lock().await;
                            sender.send((mirror_cmd, oneshot_tx)).await.unwrap();
                            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                        }
                    }
                    */

                    //从管道中读取数据，然后写入tx. tx的另一头是grpc的读取端
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
            log::warn!("namedpipe renew elapsed: {:?}", now.elapsed());
            counter += 1;
        }
    });
}

pub fn compiler_redirect_request_2(tx: std::sync::Arc<Option<tokio::sync::Mutex<tokio::sync::mpsc::Sender<(MirrorSysCall, Responders)>>>>) {

    use std::os::windows::ffi::OsStrExt;
    let os_string = std::ffi::OsString::from(r"\\.\pipe\os_operate_request_pipe");

    let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
    wchars.push(0);
    
    let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};
    let rt_ = rt.clone();
    rt.spawn(async move {
        let mut count  = 0;
        unsafe { loop {
            let pipe = winapi::um::namedpipeapi::CreateNamedPipeW(wchars.as_ptr(), 
            winapi::um::winbase::PIPE_ACCESS_INBOUND | winapi::um::winbase::PIPE_ACCESS_OUTBOUND,  
            winapi::um::winbase::PIPE_TYPE_MESSAGE | winapi::um::winbase::PIPE_READMODE_MESSAGE | winapi::um::winbase::PIPE_WAIT,
            winapi::um::winbase::PIPE_UNLIMITED_INSTANCES,
            0, 
            0,
            0, 
            std::ptr::null_mut());
            
            if !pipe.is_null() && pipe != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                
                if winapi::shared::minwindef::TRUE == winapi::um::namedpipeapi::ConnectNamedPipe(pipe, std::ptr::null_mut()) {

                    let handle = tools::ptr::HandleBox::new(pipe);
                    let _ = rt_.spawn(async move {
                        log::info!("redirect stdout log read named pipe message task start. count: {}", count);
                        let mut buffer = vec![0u8; 512];
                        let mut bytes: winapi::shared::minwindef::DWORD = 0;
                        let mut moredata = String::new();
                        loop {
                            let result = winapi::um::fileapi::ReadFile(
                                handle.get().to_owned(),
                                buffer.as_mut_ptr() as *mut _,
                                512,
                                &mut bytes,
                                std::ptr::null_mut()
                            );
            
                            if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                                let error = winapi::um::errhandlingapi::GetLastError();

                                if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                    //The pipe has been ended.
                                    break;
                                }
                                else if error == winapi::shared::winerror::ERROR_MORE_DATA {
                                    //The buffer is not enough.
                                    let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                                    moredata.push_str(&output);
                                    continue;
                                }
                                else if error == winapi::shared::winerror::ERROR_IO_PENDING {
                                    //The operation is pending.
                                    continue;
                                }
                                else {
                                    log::warn!("reaf pipe failed, error code: {}, message: {}, count: {}", error, tools::utils::get_winapi_error_message(error), count);
                                    break;
                                }
                            }
                            let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                            if moredata.is_empty() {
                                log::info!("redirect: {}", output);
                            }
                            else {
                                moredata.push_str(&output);
                                log::info!("redirect: {}", moredata);
                                moredata.clear();
                            }
                            tokio::io::stdout().write_all(format!("syscall: {}\n", output).as_bytes()).await.expect("Failed to write to stdout");
                        }
                        winapi::um::namedpipeapi::DisconnectNamedPipe(handle.get().to_owned());
                        winapi::um::handleapi::CloseHandle(handle.get().to_owned());
                        log::warn!("redirect stdout log read named pipe message task exit. count: {}", count);
                    });
                }
                else {
                    let error = winapi::um::errhandlingapi::GetLastError();
                    log::debug!("connect named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);

                    if error == winapi::shared::winerror::ERROR_NO_DATA {
        
                    }
                    else {
            
                    }
                    winapi::um::handleapi::CloseHandle(pipe);
                }
            }
            else {
                let error = winapi::um::errhandlingapi::GetLastError();
                log::error!("connect named pipe failcreate named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);
            }
            count += 1;
        }}        
    });


}

pub fn compiler_redirect_request_3(tx: std::sync::Arc<Option<tokio::sync::Mutex<tokio::sync::mpsc::Sender<(MirrorSysCall, Responders)>>>>) {
    const PIPE_NAME: &str = r"\\.\pipe\os_operate_request_pipe";
    const N: usize = 1000;

    let rt = crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone();

    let rt_ = rt.clone();
    let server = rt.spawn(async move {

        let mut server = tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME).unwrap();

        for _ in 0..N {
            // Wait for client to connect.
            log::info!("redirect xxxxxxxxxxxxxx");
            server.connect().await.unwrap();
            let mut inner = server;

            // Construct the next server to be connected before sending the one
            // we already have of onto a task. This ensures that the server
            // isn't closed (after it's done in the task) before a new one is
            // available. Otherwise the client might error with
            // `io::ErrorKind::NotFound`.
            server = tokio::net::windows::named_pipe::ServerOptions::new().create(PIPE_NAME).unwrap();

            let _ = rt_.spawn(async move {
                let mut buf = vec![0u8; 4];
                inner.read_exact(&mut buf).await.unwrap();
                log::info!("redirect: {:?}", buf);
                inner.write_all(b"pong").await.unwrap();
            });
        }
    });
}

pub fn format_mirror_command(call: &MirrorSysCall) -> String {
    let response = format!("{{\"id\": {}, \"command\": \"{}\", \"args\": {{{}}}}}",
        call.id,
        call.command,
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

        redirectdll::syscallredirect::async_connect_named_pipe();
        redirectdll::syscallredirect::async_connect_named_pipe();
        redirectdll::syscallredirect::async_connect_named_pipe();
        redirectdll::syscallredirect::async_connect_named_pipe();
        redirectdll::syscallredirect::async_connect_named_pipe();
        redirectdll::syscallredirect::async_connect_named_pipe();

        let (mut reader, mut writer) = tokio::io::split(client);

        //write meessage to pipe client
        for i in 0..10 {
            let command = MirrorSysCall {
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
        writer.shutdown().await.unwrap();

        //read message from mspc
        if let Some(mut rx) = namedpipe_rx.take() {
            for _ in 0..10 {
                let message = rx.recv().await;
                let command = message.unwrap();
                println!("received command in test: {:?}", command);
                let response = MirrorSysCall {
                    id: command.id,
                    command: "response".to_string(),
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


        let result = crate::serde_json::from_str::<MirrorSysCall>(&json);
        assert!(result.is_ok());
    }
}