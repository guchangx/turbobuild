
use std::os::windows::io::AsRawHandle;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use windows_sys::Win32 as win;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct MirrorSysCall {
    pub cid: u32,
    pub api: String,
    pub args: std::collections::HashMap<String, String>,
}

pub struct CHANNEL {
    pub grpc_to_namedpipe_tx: std::sync::Arc<tokio::sync::broadcast::Sender<MirrorSysCall>>,
    pub grpc_to_namedpipe_rx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::broadcast::Receiver<MirrorSysCall>>>,
}

pub static GRPC_TO_NAMEDPIPE_CHANNEL: std::sync::LazyLock<CHANNEL> = std::sync::LazyLock::new(|| {

    let (tx, rx) = tokio::sync::broadcast::channel(1024);
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
        let makeserver = |first| { 
            tokio::net::windows::named_pipe::ServerOptions::new()
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

        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_ = notify.clone();
        let mut joinset = tokio::task::JoinSet::new();
        
        for _ in 1..std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).max(8) {
            let notify = notify_.clone();
            joinset.spawn(async move {
                notify.notified().await;
                let server = makeserver(false);
                server.connect().await.expect("waiting failed to connect to named pipe.");
                server
            });
        }

        joinset.spawn(async move {
            let server = makeserver(true);
            notify.notify_waiters();
            server.connect().await.expect("waiting failed to connect to named pipe.");
            server
        });

        while let Some(joinserver) = joinset.join_next().await {
            let server = match joinserver {
                Ok(s) => {
                    joinset.spawn(async move {
                        let server = makeserver(false);
                        server.connect().await.expect("server failed to connect to named pipe");
                        server
                    });
                    s
                },
                Err(e) => {
                    log::error!("failed to create named pipe server: {}", e);
                    joinset.spawn(async move {
                        let server = makeserver(false);
                        server.connect().await.expect("server failed to connect to named pipe");
                        server
                    });
                    continue;
                }
            };

            let mut pid: u32 = 0;
            unsafe {
                win::System::Pipes::GetNamedPipeClientProcessId(server.as_raw_handle() as _, &mut pid as *mut u32);
            }
            
            let (mut reader, mut writer) = tokio::io::split(server);

            log::info!("namedpipe connected counter: {:?} success, client pid: {:?}", counter, pid);
            //return the syscall response to caller in func.rs
            let closed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let closed_ = closed.clone();

            rt_.spawn(async move {
                let mut rx = { GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_rx.lock().await.resubscribe() };
                while !closed.load(std::sync::atomic::Ordering::Relaxed) {
                    match rx.recv().await {
                        Ok(response) => {
                            if response.cid / 10000 == pid {
                                let response = format_mirror_syscall(&response);
                                match writer.write_all(response.as_bytes()).await {
                                    Ok(_) => {
                                    },
                                    Err(err) => {
                                        log::error!("failed to write mirror syscall response to namedpipe. {:?} err: {}", response, err);
                                        break;
                                    }
                                }
                            }
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("cocrew namedpipe writer for pid {} lagged by {} messages, some syscall responses lost!", pid, n);
                            continue;
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            log::warn!("mirror syscall response broadcast channel closed for pid {}.", pid);
                            break;
                        }
                    }
                }
                writer.shutdown().await.unwrap();
                log::trace!("cocrew syscall namedpipe writer end {}", pid);
            });

            let _ = rt_.spawn(async move {
                loop {
                    //read command form namedpipe and send it to grpc.
                    let mut data = vec![0; 1024];
                    match reader.read(&mut data).await {
                        Ok(size) => {
                            if size > 0 as usize {
                                let message = String::from_utf8_lossy(&data[..size]);
                                if let Ok(mirror_cmd) = crate::serde_json::from_str::<MirrorSysCall>(&message)
                                    .map_err(|e| {
                                        log::error!("failed to parse mirror syscall: {}", e);
                                    }) {

                                    if let Some(sender) = crate::communicate::unpackager::NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_tx.as_ref() {
                                        sender.try_send(mirror_cmd.clone()).inspect(|_|{
                                        }).unwrap_or_else(|e| {
                                            log::error!("failed to try_send mirror syscall: {} {:?}", e, mirror_cmd);
                                        });
                                    }
                                }
                            }
                            else {
                                log::warn!("no data received from syscall namedpipe, disconnecting.");
                                break;
                            }
                        },
                        Err(e) => {
                            log::error!("failed to read from syscall namedpipe: {}", e);
                            break;
                        }
                    }
                }
                closed_.store(true, std::sync::atomic::Ordering::Relaxed);
                log::trace!("cocrew syscall namedpipe reader end. client pid: {}", pid);
            });
            
            counter += 1;
        }
    });
}

pub fn format_mirror_syscall(call: &MirrorSysCall) -> String {
    let response = format!("{{\"cid\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
        call.cid,
        call.api,
        call.args.iter()
            .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(", "));
    return response;
}

pub fn compiler_redirect_syscall_2() {
    use std::os::windows::ffi::OsStrExt;
    let os_string = std::ffi::OsString::from(r"\\.\pipe\os_operate_request_pipe");

    let mut name_wchars = os_string.encode_wide().collect::<Vec<_>>();
    name_wchars.push(0);

    let mut count  = 0;
    let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};
    let rt_ = rt.clone();
    rt_.spawn_blocking(move || { unsafe {

        let mut pipe = win::System::Pipes::CreateNamedPipeW(name_wchars.as_ptr(), 
        win::Storage::FileSystem::PIPE_ACCESS_DUPLEX, 
        win::System::Pipes::PIPE_TYPE_MESSAGE | win::System::Pipes::PIPE_READMODE_MESSAGE | win::System::Pipes::PIPE_WAIT,
        win::System::Pipes::PIPE_UNLIMITED_INSTANCES,
        65536,
        65536,
        0,
        std::ptr::null_mut());

        loop {
        
        if !pipe.is_null() && pipe != win::Foundation::INVALID_HANDLE_VALUE {
            
            if win::Foundation::TRUE == win::System::Pipes::ConnectNamedPipe(pipe, std::ptr::null_mut()) {

                let handle = tools::ptr::HandleBox::new(pipe);
                let handle_ = handle.clone();

                pipe = win::System::Pipes::CreateNamedPipeW(name_wchars.as_ptr(),
                win::Storage::FileSystem::PIPE_ACCESS_DUPLEX, 
                win::System::Pipes::PIPE_TYPE_MESSAGE | win::System::Pipes::PIPE_READMODE_MESSAGE | win::System::Pipes::PIPE_WAIT,
                win::System::Pipes::PIPE_UNLIMITED_INSTANCES,
                65536,
                65536,
                0,
                std::ptr::null_mut());

                let mut pid: u32 = 0;
                win::System::Pipes::GetNamedPipeClientProcessId(handle.get().to_owned() as _, &mut pid as *mut u32);
                log::info!("namedpipe connected count {} success, client pid: {}", count, pid);

                let _ = rt.spawn_blocking(move || {

                    let mut buffer = vec![0u8; 512];
                    let mut bytes: u32 = 0;
                    loop {
                        let mut total_bytes_avail: u32 = 0;
                        let result = win::System::Pipes::PeekNamedPipe(
                            handle.get().to_owned() as _,
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            &mut total_bytes_avail,
                            std::ptr::null_mut(),
                        );

                        if result == win::Foundation::FALSE {
                            let error = win::Foundation::GetLastError();
                            log::warn!("peek pipe failed, error code: {}, message: {}, count: {} client pid: {}", error, tools::utils::get_winapi_error_message(error), count, pid);
                            break;
                        }
                        else if total_bytes_avail == 0 {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            continue;
                        }

                        let result = win::Storage::FileSystem::ReadFile(
                            handle.get().to_owned() as _,
                            buffer.as_mut_ptr() as *mut _,
                            512,
                            &mut bytes,
                            std::ptr::null_mut()
                        );
        
                        if result == win::Foundation::FALSE {
                            let error = win::Foundation::GetLastError();
                            log::warn!("reaf pipe failed, error code: {}, message: {}, count: {}", error, tools::utils::get_winapi_error_message(error), count);
                            break;
                        }
                        else if bytes == 0 {
                            log::warn!("read 0 bytes from pipe, client disconnected, count: {}", count);
                            break;
                        }
                        else {
                            let message = String::from_utf8_lossy(&buffer[..bytes as usize]);
                            log::info!("received mirror syscall: {}", message);

                            if let Ok(mirror_cmd) = crate::serde_json::from_str::<MirrorSysCall>(&message)
                                .map_err(|e| { log::error!("failed to parse mirror syscall: {}", e); }) 
                            {
                                if let Some(sender) = crate::communicate::unpackager::NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_tx.as_ref() {
                                    sender.try_send(mirror_cmd.clone()).inspect(|_|{
                                        log::trace!("send mirror syscall to grpc: {:?}", mirror_cmd);
                                    }).unwrap_or_else(|e| {
                                        log::error!("failed to try_send mirror syscall: {} {:?}", e, mirror_cmd);
                                    });
                                }
                            }
                        }
                    }
                    win::System::Pipes::DisconnectNamedPipe(handle.get().to_owned() as _);
                    win::Foundation::CloseHandle(handle.get().to_owned() as _);
                    log::warn!("redirect syscall read named pipe message task exit. count: {} client pid: {}", count, pid);
                });

                /* 
                let process = win::System::Threading::GetCurrentProcess();
                let dup_pipe: *mut core::ffi::c_void = std::ptr::null_mut();
                let ret = win::Foundation::DuplicateHandle(
                    process,
                    pipe,
                    process,
                    &dup_pipe as *const _ as *mut _,
                    0,
                    win::Foundation::FALSE,
                    win::Foundation::DUPLICATE_SAME_ACCESS
                );
                */

                let rt_ = rt.clone();
                let _ = rt.spawn_blocking( move || {
                    let mut rx = rt_.block_on(async move {
                        let rx = { GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_rx.lock().await.resubscribe() };
                        rx
                    });
                    log::trace!("start mirror syscall response task for pid: {}", pid);
                    loop {
                        if let Ok(response) = rx.blocking_recv() {
                            log::trace!("mirror syscall response recv: pid: {:?} {:?}", pid, response);
                            if response.cid / 10000 == pid {
                                let response = format_mirror_syscall(&response);
                                log::debug!("received grpc response: {:?}", response);

                                let mut bytes: u32 = 0;
                                let result = win::Storage::FileSystem::WriteFile(
                                    handle_.get().to_owned() as _,
                                    response.as_bytes().as_ptr() as *const u8,
                                    response.len() as u32,
                                    &mut bytes,
                                    std::ptr::null_mut()
                                );
                                if result == win::Foundation::FALSE {
                                    let error = win::Foundation::GetLastError();
                                    log::error!("failed to write mirror syscall response to pipe. {:?} err: {}, message: {}", response, error, tools::utils::get_winapi_error_message(error));
                                }
                                else {
                                    log::info!("success sent mirror syscall response. send size: {} length: {} {:?}", bytes, response.as_bytes().len(), response);
                                }
                            }
                        }
                        else {
                            log::warn!("mirror syscall response channel closed, dropped receiver.");
                            break;
                        }
                    }
                    log::trace!("stop mirror syscall response task for pid: {}", pid);
                });
            }
            else {
                let error = win::Foundation::GetLastError();
                log::debug!("connect named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);
                win::Foundation::CloseHandle(pipe as _);
                break;
            }
        }
        else {
            let error = win::Foundation::GetLastError();
            log::error!("connect named pipe failcreate named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);
            break;
        }
        count += 1;
    }}});

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
                Err(e) if e.raw_os_error() == Some(win::Foundation::ERROR_PIPE_BUSY as i32) => (),
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
                cid: i,
                api: "test".to_string(),
                args: std::collections::HashMap::new(),
            };

            let message = format!("{{\"cid\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
                    command.cid,
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
                    cid: command.cid,
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
            cid: 0,
            api: "NtQueryDirectoryFile".to_string(),
            args: args,
        };

        let json = format!("{{\"cid\": {}, \"api\": \"{}\", \"args\": {{{}}}}}",
                    command.cid,
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