
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tokio::io::AsyncSeekExt;

pub mod pack {
    include!("../../proto/pack.rs");
}

#[derive(Default, Clone)] 
pub struct TransmitArchive {
    pub path: String,
    pub offset: i64,
    pub content: bytes::Bytes,
}

type SharedTransmitFsHandle = std::sync::Arc<tokio::sync::Mutex<tokio::fs::File>>;
type TransmitFileTable = std::sync::Arc<
    tokio::sync::RwLock<std::collections::HashMap<String, SharedTransmitFsHandle>>,
>;

type TransmitFileTableWinNative = std::sync::Arc<
    tokio::sync::RwLock<std::collections::HashMap<String, std::sync::Arc<tools::ptr::HandleBox>>>,
>;

#[repr(C)]
struct NativeWriteRequest {
    // OVERLAPPED must be the first field because the completion thread casts
    // the OVERLAPPED pointer back to NativeWriteRequest.
    overlapped: windows_sys::Win32::System::IO::OVERLAPPED,
    file: std::sync::Arc<tools::ptr::HandleBox>,
    content: bytes::Bytes,
    offset: u64,
}

unsafe impl Send for NativeWriteRequest {}
unsafe impl Sync for NativeWriteRequest {}

#[derive(Clone)]
pub struct Sender {
    client: pack::communicate_client::CommunicateClient<tonic::transport::Channel>,
    host: String,
    runtime: Option<std::sync::Arc<tokio::runtime::Handle>>,
    transmit_archive_tx: tokio::sync::mpsc::Sender<TransmitArchive>,
}

pub struct CommandArgs {

}

pub struct FileArgs {
    
}

pub struct PrecompiledFile<'a> {
    pub solution: String,
    pub project: String,
    pub file: String,
    pub compiler: String,
    pub working_dir: String,
    pub variety: String,
    pub commands: Vec<String>,
    pub content: std::borrow::Cow<'a, [u8]>,
}

pub struct SourcesFile<'a> {
    pub solution: String,
    pub project: String,
    pub file: String,
    pub compiler: String,
    pub working_dir: String,
    pub variety: String,
    pub commands: Vec<String>,
    pub content: std::borrow::Cow<'a, [u8]>,
    pub envs: std::collections::HashMap<String, String>,
    pub presyncfiles: Vec<String>,
}

pub enum FileType {
    Unknown = 0,
    SourceFiles = 1,
    PrecompiledSrcFiles = 2,
    ToolChain = 3,
    Kits = 4,
    SyncTaskCount = 5,
    Finish = 6,
}

pub struct ArchiveArgs {
    pub file_type: FileType,
    pub solution: String,
    pub project: String,
    pub name: String,
    pub path: String,
    pub content: bytes::Bytes,
}

pub struct ArchiveStreamArgs {
    pub rx: tokio::sync::mpsc::Receiver<super::packager::ArchiveArgs>,
    pub callback: Box<dyn Fn() + Send + Sync>,
}

pub enum SenderType<'a> {
    Command(CommandArgs),
    Archive(ArchiveArgs),
    ArchiveStream(ArchiveStreamArgs),
    Compile(SourcesFile<'a>, crate::compiler::model::OutputCallback),
    CheckResource,
}

pub struct CommandRecv {
    pub status: bool,
    pub message: String,
}

pub struct ArchiveRecv {
    pub status: bool,
    pub message: String,
}

pub struct CompileRecv {
    pub status: u32,
    pub out: Vec<u8>,
    pub err: Vec<u8>
}

pub enum ReceiverType {
    Command(CommandRecv),
    Archive(ArchiveRecv),
    Compile(CompileRecv),
    None,
}

static TRANSMIT_ARCHIVE_CHANNEL: std::sync::OnceLock<tokio::sync::mpsc::Sender<TransmitArchive>> = std::sync::OnceLock::new();

impl Sender {
    pub async fn new(addr: &str, runtime: Option<&std::sync::Arc<tokio::runtime::Handle>>) -> Self {
        let mut host = "localhost"; 
        if !addr.is_empty() {
            host = addr;
        }
        
        let channel = tonic::transport::Endpoint::from_shared(std::format!("http://{}:19302", host)).unwrap()
            .initial_stream_window_size(32 * 1024 * 1024)
            .initial_connection_window_size(64 * 1024 * 1024)
            .connect_timeout(std::time::Duration::from_secs(30))
            .connect()
            .await
            .expect(&format!("Failed to connect to the {}:19302 server", host));

        let client = pack::communicate_client::CommunicateClient::new(channel)
            .max_decoding_message_size(1024 * 1024 * 180 * 2)
            .max_encoding_message_size(1024 * 1024 * 180 * 2);

        let sender = Sender {
            client,
            host: host.to_string(),
            runtime: runtime.cloned(),
            #[cfg(target_os = "windows")]
            transmit_archive_tx: Self::transmit_archive_sender_win_native(),
            #[cfg(target_os = "macos")]
            transmit_archive_tx: Self::transmit_archive_sender(),
        };

        return sender;
    }
    
    pub async fn dist<'a>(&mut self, sender_type: SenderType<'a>) -> ReceiverType {
        
        match sender_type {
            SenderType::Command(_args) => {
                self.redirect_net_command().await;

                let result = CommandRecv {
                    status: true,
                    message: "".to_string(),
                };
                return ReceiverType::Command(result);
            },
            SenderType::Archive(args) => {
                let result= self.dist_archive(args).await;
                return ReceiverType::Archive(result);
            },
            SenderType::ArchiveStream(args) => {
                let result= self.dist_archive_stream(args).await;
                return ReceiverType::Archive(result);
            },
            SenderType::Compile(args, callback) => {
                let result = self.dist_compile(args, callback).await;
                return ReceiverType::Compile(result);
            },
            _ => {
                return ReceiverType::None;
            }
        }
    }
    
    async fn dist_archive(&mut self, args: ArchiveArgs) -> ArchiveRecv {

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
        let request = pack::FileTrRequest {
            file_type: args.file_type as i32,
            solution: args.solution.clone(),
            project: args.project.clone(),
            name: args.name,
            path:  args.path,
            content: args.content,
        };

        if let Err(err) = tx.send(request).await {
            log::error!("transmit file error: {:?}", err);
            let result = ArchiveRecv {
                status: false,
                message: "".to_string(),
            };
            return result;
        };

        let request = pack::FileTrRequest {
            file_type: pack::FileType::Finish as i32,
            solution: String::new(),
            project: String::new(),
            name: String::new(),
            path: String::new(),
            content: bytes::Bytes::new(),
        };

        if let Err(err) = tx.send(request).await {
            log::error!("transmit file finish error: {:?}", err);
            let result = ArchiveRecv {
                status: false,
                message: "".to_string(),
            };
            return result;
        };
        
        drop(tx);

        match self.to_owned().client.transmit_file(request_stream).await {
            Ok(response) => {
                
                let host = self.host.clone();

                let mut response_stream = response.into_inner();
                while let Some(stream) = response_stream.next().await {
                    match stream {
                        Ok(stream) => {
                            log::debug!("transmit file {} response code: {}, message: {}", host, stream.error_code, stream.error_message);
                        }
                        Err(err) => {
                            log::error!("transmit file {} failed: {:?}", host, err);
                            break;
                        }
                    }
                };
                
                let result = ArchiveRecv {
                    status: true,
                    message: "".to_string(),
                };
                return result;
            },
            Err(err) => {
                log::error!("transmit file  {} failed: {:?}", self.host, err);
                let result = ArchiveRecv {
                    status: false,
                    message: "".to_string(),
                };
                return result;
            }
        }
    }

    async fn dist_archive_stream(&mut self, args: ArchiveStreamArgs) -> ArchiveRecv {

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

        let mut receiver = args.rx;
        let callback = args.callback;

        let handle = self.runtime.as_ref().map(|runtime| runtime.spawn(async move {

            while let Some(archive) = receiver.recv().await {
                let request = pack::FileTrRequest {
                    file_type: archive.file_type as i32,
                    solution: archive.solution.clone(),
                    project: archive.project.clone(),
                    name: archive.name,
                    path: archive.path,
                    content: archive.content,
                };
        
                if let Err(err) = tx.send(request).await {
                    log::error!("transmit file error: {:?}", err);
                };
            }

            drop(tx);
        }));

        let mut result = ArchiveRecv {
            status: true,
            message: "".to_string(),
        };

        match self.to_owned().client.transmit_file(request_stream).await {
            Ok(response) => {
                
                let host = self.host.clone();

                let mut response_stream = response.into_inner();
                while let Some(stream) = response_stream.next().await {
                    match stream {
                        Ok(stream) => {
                            //log::debug!("transmit file {} response code: {}, message: {}", host, stream.error_code, stream.error_message);
                            
                            if !stream.path.is_empty() {
                                let file = TransmitArchive {
                                    path: stream.path,
                                    offset: stream.offset,
                                    content: stream.content,
                                };

                                if let Err(err) = self.transmit_archive_tx.send(file).await {
                                    log::error!("queue received archive save request failed: {:?}", err);
                                }
                            }
                        }
                        Err(err) => {
                            log::error!("transmit file {} failed: {:?}", host, err);
                            break;
                        }
                    }
                };

                log::debug!("transmit file {} completed.", host);
                callback();
            },
            Err(err) => {
                log::error!("transmit file {} failed: {:?}", self.host, err);
                result.status = false;
            }
        };

        if let Some(handle) = handle {
            let _ = handle.await.unwrap();
        }

        return result;
    }

    async fn dist_compile(&mut self, compile: SourcesFile<'_>, output_callback: crate::compiler::model::OutputCallback) -> CompileRecv {
        let project = compile.project.clone();
        let request = tonic::Request::new(pack::CompileTrRequest {
            solution: compile.solution,
            project: compile.project,
            file: compile.file,
            compiler: compile.compiler,
            working_dir: compile.working_dir,
            variety: compile.variety,
            commands: compile.commands.clone(),
            envs: compile.envs.iter().map(|(k,v)| pack::Envs {
                key: k.clone(),
                value: v.clone(),
            }).collect(),
            
            content: compile.content.to_vec(),
            presyncfiles: compile.presyncfiles,
        });

        log::debug!("send compiled sourcefile request: {:?}", compile.commands.clone());

        let response = self.to_owned().client.transmit_task(request).await;

        let mut recv = CompileRecv {
            status: 0,
            out: Vec::new(),
            err: Vec::new(),
        };

        match response {
            Ok(response) => {
                let now = std::time::Instant::now();
                let (tx, rx) = tokio::sync::mpsc::channel::<Vec<pack::IntermediateResult>>(128);
                let mut myself = self.clone();
                let project_ = project.clone();
                let save_compile_ouput_handle = self.runtime.as_ref().map(|runtime| {
                    let handle = runtime.spawn(async move {
                        myself.save_compile_ouput_form_channel(rx, &project_).await;
                    });
                    return handle;
                });

                let mut stream = response.into_inner();
                while let Some(inner) = stream.next().await {
                    match inner {
                        Ok(response) => {
                            if response.status == 0 {
                                if response.progress == pack::CompileProgress::Filetransfer as i32 {

                                }
                                else if response.progress == pack::CompileProgress::Compilestart as i32 {
                                }
                                else if response.progress == pack::CompileProgress::Compiling as i32 {

                                    if !response.results.is_empty() {
                                        log::trace!("{:?} compiling receive compiled sourcefile response: {:?}", project, response.results.iter().map(|item| item.file.clone()).collect::<Vec<_>>());

                                        tx.send(response.results).await.unwrap_or_else(|err| {
                                            log::error!("send compiled sourcefile response to save failed: {:?}", err);
                                        });                                        
                                    }

                                    let output = crate::compiler::model::CompilerOutput {
                                        status: 0,
                                        out: std::sync::Arc::new(response.out),
                                        err: std::sync::Arc::new(response.err),
                                    };
                                    
                                    // send one compiled done file to buildassist
                                    let callback = (output_callback)(output);
                                    let callback = Box::into_pin(callback);
                                    let _ = callback.await;
                                }
                                else if response.progress == pack::CompileProgress::Compiledone as i32 {
            
                                    log::debug!("{:?} compiled sourcefile done response out: {:?}", project, String::from_utf8_lossy(&response.out));
                                    recv.out = response.out;
                                
                                    log::debug!("{:?} compiled sourcefile done response err: {:?}", project, String::from_utf8_lossy(&response.err));
                                    recv.err = response.err;
                                    
                                    recv.status = response.status;
                                    
                                    let mut myself = self.clone();
                                    let project_ = project.clone();
                                    self.runtime.clone().unwrap().spawn(async move {
                                        let runtime = myself.runtime.clone().unwrap();
                                        myself.save_compile_output(response.results, &runtime, &project_).await;
                                    });
                                    
                                }
                                else {
                                    
                                }
                            }
                            else {
                                log::debug!("{:?} compiled sourcefile failed response out: {:?}", project, String::from_utf8_lossy(&response.out));
                                log::debug!("{:?} compiled sourcefile failed response err: {:?}", project, String::from_utf8_lossy(&response.err));
                                recv.status = response.status;
                                recv.out = response.out;
                                recv.err = response.err;
                                break;
                            }
                        },
                        Err(err) => {
                            log::error!("{:?} send compiled sourcefile receive response failed. {}", project, err);
                            recv.status = 1;
                            recv.err = err.to_string().into_bytes();
                            break;
                        },
                    }
                }

                drop(tx);
                if let Some(handle) = save_compile_ouput_handle {
                    let _ = handle.await;
                }

                log::info!("send compiled sourcefile receive response done. {}. elapsed: {:?}", project, now.elapsed());
            }
            Err(err) => {
                log::warn!("send compiled sourcefile failed: {:?} {}", err, project);
                recv.status = 1;
                recv.err = err.to_string().into_bytes();
            }
        }
        return recv;
    }

    async fn save_compile_output(&mut self, results: Vec<pack::IntermediateResult>, runtime: &std::sync::Arc<tokio::runtime::Handle>, project: &str) {
        let mut handles = Vec::new();
        let len = results.len();
        for result in results {

            let handle = runtime.spawn(async move {
                log::debug!("save compile result: {:?}", result.file);
                match tokio::fs::OpenOptions::new().write(true).create(true).open(&result.file).await {
                    Ok(mut file) => {
                        file.write_all(&result.content).await.unwrap();

                        //let mut writer = tokio::io::BufWriter::new(file);
                        //writer.write_all(&result.content).await.unwrap();
                        //writer.flush().await.unwrap();
                    },
                    Err(err) => {
                        log::error!("create file failed: {}, path: {}", err, result.file);
                    }
                }
            });
            handles.push(handle);
        }

        for hande in handles {
            match hande.await {
                Ok(_) => {},
                Err(err) => {
                    log::error!("save compile output failed: {:?}", err);
                }
            }
        }
        log::info!("{} save compile output done. file count: {}", project, len);
    }

    async fn save_compile_ouput_form_channel(&mut self, mut stream: tokio::sync::mpsc::Receiver<Vec<pack::IntermediateResult>>, project: &str) {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(64));
        while let Some(results) = stream.recv().await {
            for result in results {
                log::debug!("save compile result: {:?}", result.file);
                let permit = semaphore.clone().acquire_owned().await;

                self.runtime.as_ref().map(|runtime| {

                    let _handle = runtime.spawn(async move {
                        let _permit = permit;
                        match tokio::fs::OpenOptions::new().write(true).create(true).open(&result.file).await {
                            Ok(mut file) => {
                                file.write_all(&result.content).await.unwrap();
                            },
                            Err(err) => {
                                log::error!("save compile output create file failed: {}, path: {}", err, result.file);
                            }
                        }
                    });
                });

            }
        }
    }

    async fn redirect_net_command(&mut self) {
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        log::debug!("transmit redirect net start addr: {}.", self.host);

        match self.to_owned().client.transmit_syscall(request_stream).await {
            Ok(response) => {
                
                let host = self.host.clone();

                let mut response_stream = response.into_inner();
                while let Some(stream) = response_stream.next().await {
                    match stream {
                        Ok(stream) => {
                            let local = crate::procemirror::filesystem::route_file_system_operation(stream); //700 µs

                            if let Err(err) = tx.send(local.clone()).await {
                                log::error!("transmit redirect net error: {:?}", err);
                            }
                            else {
                            } 
                        }
                        Err(err) => {
                            log::error!("transmit redirect net {} failed: {:?}", host, err);
                            break;
                        }
                    }
                };
                drop(tx);
                log::debug!("transmit firedirect netle {} completed.", host);
                crate::communicate::distributor::CONNECTED_ADDRS.lock().await.retain(|item| item != &host);
            },
            Err(err) => {
                log::error!("transmit redirect net {} failed: {:?}", self.host, err);
            }
        };
    }

    fn transmit_archive_sender() -> tokio::sync::mpsc::Sender<TransmitArchive> {
        TRANSMIT_ARCHIVE_CHANNEL
            .get_or_init(|| {
                let (sender, receiver) = tokio::sync::mpsc::channel(512);
                let receiver = std::sync::Arc::new(tokio::sync::Mutex::new(receiver));

                let fshandle: TransmitFileTable = std::sync::Arc::new(tokio::sync::RwLock::new(
                    std::collections::HashMap::new(),
                ));

                let worker_count: usize = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).max(32) / 2;
                for _ in 0..worker_count {
                    let receiver = receiver.clone();
                    let fshandle = fshandle.clone();
                    tokio::spawn(async move {
                        Self::multiworker_save_transmit_archives(
                            receiver,
                            fshandle,
                        )
                        .await;
                    });
                }

                sender
            })
            .clone()
    }

    async fn get_or_open_transmit_file(
        fshandle: &TransmitFileTable,
        path: &str,
    ) -> std::io::Result<SharedTransmitFsHandle> {

        let mut files = fshandle.write().await;

        if let Some(state) = files.get(path) {
            return Ok(state.clone());
        }
        else {
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .await?;
    
            let state = std::sync::Arc::new(tokio::sync::Mutex::new(file));
    
            files.insert(path.to_owned(), state.clone());
            return Ok(state)
        }
    }

    async fn multiworker_save_transmit_archives(
        receiver: std::sync::Arc<
            tokio::sync::Mutex<tokio::sync::mpsc::Receiver<TransmitArchive>>,
        >,
        fshandle: TransmitFileTable,
    ) {
        loop {

            let (transmit_file_stream, file) = {
                let mut receiver = receiver.lock().await;
                let Some(transmit_file_stream) = receiver.recv().await else {
                    break;
                };

                //chunk
                if transmit_file_stream.offset >= 0 && !transmit_file_stream.content.is_empty() {
                    match Self::get_or_open_transmit_file(
                        &fshandle,
                        &transmit_file_stream.path,
                    )
                    .await
                    {
                        Ok(file) => (transmit_file_stream, Some(file)),
                        Err(error) => {
                            log::error!(
                                "open file chunk {} failed: {:?}",
                                transmit_file_stream.path,
                                error
                            );
                            (transmit_file_stream, None)
                        }
                    }
                }
                //last chunk
                else if transmit_file_stream.offset == -1 && transmit_file_stream.content.is_empty() {
                    let handle = fshandle.write().await.remove(&transmit_file_stream.path);
                    (transmit_file_stream, handle)
                } 
                //completefile
                else {
                    (transmit_file_stream, None)
                }
            };

            //save completefile
            if transmit_file_stream.offset == -1 && !transmit_file_stream.content.is_empty() {
                match tokio::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&transmit_file_stream.path)
                    .await
                {
                    Ok(mut file) => {
                        if let Err(error) = file.write_all(&transmit_file_stream.content).await {
                            log::error!(
                                "result file save worker write {} failed: {:?}",
                                transmit_file_stream.path,
                                error
                            );
                        }
                        else {
                            log::debug!(
                                "result file save worker write {} done.",
                                transmit_file_stream.path
                            );
                        }
                    }
                    Err(error) => {
                        log::error!(
                            "result file save worker open {} failed: {:?}",
                            transmit_file_stream.path,
                            error
                        );
                    }
                }
                continue;
            }
            else {
                if let Some(file) = file {
                    
                    let mut file = file.lock().await;
                    
                    if transmit_file_stream.content.is_empty() {
                        if let Err(err) = file.flush().await {
                            log::error!(
                                "flush file chunk {} at offset {} failed: {:?}",
                                transmit_file_stream.path,
                                transmit_file_stream.offset,
                                err
                            );
                        }
                        else {
                            log::debug!(
                                "flush file chunk {} done.",
                                transmit_file_stream.path
                            );
                        }
                        continue;
                    }
                    else {
                        log::debug!("seek file chunk {} offset: {} length: {}", transmit_file_stream.path, transmit_file_stream.offset, transmit_file_stream.content.len());
                        if let Err(error) = file
                            .seek(std::io::SeekFrom::Start(transmit_file_stream.offset as u64))
                            .await
                        {
                            log::error!(
                                "seek file chunk {} at offset {} failed: {:?}",
                                transmit_file_stream.path,
                                transmit_file_stream.offset,
                                error
                            );
                            continue;
                        }
                        if let Err(error) = file.write_all(&transmit_file_stream.content).await {
                            log::error!(
                                "write file chunk {} at offset {} failed: {:?}",
                                transmit_file_stream.path,
                                transmit_file_stream.offset,
                                error
                            );
                        }
                    }
                };
            }
        }
    }

    fn transmit_archive_sender_win_native() -> tokio::sync::mpsc::Sender<TransmitArchive> {
        TRANSMIT_ARCHIVE_CHANNEL
            .get_or_init(|| {
                let (sender, receiver) = tokio::sync::mpsc::channel(512);
                let receiver = std::sync::Arc::new(tokio::sync::Mutex::new(receiver));

                let fshandle: TransmitFileTableWinNative = std::sync::Arc::new(tokio::sync::RwLock::new(
                    std::collections::HashMap::new(),
                ));

                let iocp_handle = unsafe {
                    windows_sys::Win32::System::IO::CreateIoCompletionPort(
                        windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
                        std::ptr::null_mut(),
                        0,
                        0,
                    )
                };
                if iocp_handle.is_null() {
                    panic!(
                        "CreateIoCompletionPort failed: {:?}",
                        std::io::Error::last_os_error()
                    );
                }

                let iocp = std::sync::Arc::new(tools::ptr::HandleBox::new(iocp_handle));

                let completion_iocp = iocp.clone();
                std::thread::Builder::new()
                    .name("crew-file-iocp".to_string())
                    .spawn(move || Self::native_iocp_completion_loop(completion_iocp))
                    .expect("spawn native file IOCP completion thread failed");

                let worker_count: usize = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).max(32) / 2;
                for _ in 0..worker_count {
                    let receiver = receiver.clone();
                    let fshandle = fshandle.clone();
                    let iocp = iocp.clone();
                    tokio::spawn(async move {
                        Self::multiworker_save_transmit_archives_win_native(
                            receiver,
                            fshandle,
                            iocp,
                        )
                        .await;
                    });
                }

                sender
            })
            .clone()
    }

    fn multiworker_save_transmit_archives_win_native(
        receiver: std::sync::Arc<
            tokio::sync::Mutex<tokio::sync::mpsc::Receiver<TransmitArchive>>,
        >,
        fshandle: TransmitFileTableWinNative,
        iocp: std::sync::Arc<tools::ptr::HandleBox>,
    ) -> impl std::future::Future<Output = ()> {
        async move {
            loop {
                let (transmit_file, file) = {
                    let mut receiver = receiver.lock().await;
                    let Some(transmit_file) = receiver.recv().await else {
                        break;
                    };
                    
                    drop(receiver);

                    if transmit_file.offset >= 0 && !transmit_file.content.is_empty() {
                        match Self::get_or_open_transmit_file_win_native(
                            &fshandle,
                            &iocp,
                            &transmit_file.path,
                        )
                        .await
                        {
                            Ok(file) => (transmit_file, Some(file)),
                            Err(error) => {
                                log::error!(
                                    "native open file chunk {} failed: {:?}",
                                    transmit_file.path,
                                    error
                                );
                                (transmit_file, None)
                            }
                        }
                    } else if transmit_file.offset == -1 && transmit_file.content.is_empty() {
                        if let Some(handle) = fshandle.write().await.remove(&transmit_file.path) {
                            if let Ok(file) = std::sync::Arc::try_unwrap(handle) {
                                unsafe {
                                    windows_sys::Win32::Foundation::CloseHandle(*file.get());
                                }
                            }
                        }
                        (transmit_file, None)
                    } else {
                        (transmit_file, None)
                    }
                };

                if transmit_file.offset == -1 && !transmit_file.content.is_empty() {
                    match Self::open_win_native_transmit_file(
                        &iocp,
                        &transmit_file.path,
                        windows_sys::Win32::Storage::FileSystem::CREATE_ALWAYS,
                    ) {
                        Ok(file) => {
                            if let Err(error) = Self::submit_native_write(
                                file,
                                0,
                                transmit_file.content,
                            ) {
                                log::error!(
                                    "native write complete file {} failed: {:?}",
                                    transmit_file.path,
                                    error
                                );
                            }
                        }
                        Err(error) => {
                            log::error!(
                                "native open complete file {} failed: {:?}",
                                transmit_file.path,
                                error
                            );
                        }
                    }
                    continue;
                }

                let Some(file) = file else {
                    continue;
                };

                log::info!("submitting native write for file {} at offset {} with content length {}", transmit_file.path, transmit_file.offset, transmit_file.content.len());
                if let Err(error) = Self::submit_native_write(
                    file,
                    transmit_file.offset as u64,
                    transmit_file.content,
                ) {
                    log::error!(
                        "native write file {} at offset {} failed: {:?}",
                        transmit_file.path,
                        transmit_file.offset,
                        error
                    );
                }
            }
        }
    }

    async fn get_or_open_transmit_file_win_native(
        fshandle: &TransmitFileTableWinNative,
        iocp: &std::sync::Arc<tools::ptr::HandleBox>,
        path: &str,
    ) -> std::io::Result<std::sync::Arc<tools::ptr::HandleBox>> {

        {
            let files = fshandle.read().await;
            if let Some(state) = files.get(path) {
                return Ok(state.clone());
            }
        }

        let mut files = fshandle.write().await;

        if let Some(state) = files.get(path) {
            return Ok(state.clone());
        }

        let file = Self::open_win_native_transmit_file(
            iocp,
            path,
            windows_sys::Win32::Storage::FileSystem::CREATE_ALWAYS,
        )?;

        files.insert(path.to_owned(), file.clone());
        Ok(file)
    }

    fn open_win_native_transmit_file(
        iocp: &std::sync::Arc<tools::ptr::HandleBox>,
        path: &str,
        creation_disposition: u32,
    ) -> std::io::Result<std::sync::Arc<tools::ptr::HandleBox>> {
        use std::os::windows::ffi::OsStrExt;

        let path = if let Some(path) = path.strip_prefix("\\??\\") {
            path.to_owned()
        } 
        else {
            path.to_owned()
        };

        let path = std::ffi::OsStr::new(&path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>();

        let handle = unsafe {
            windows_sys::Win32::Storage::FileSystem::CreateFileW(
                path.as_ptr(),
                windows_sys::Win32::Foundation::GENERIC_WRITE,
                windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                    | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE
                    | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE,
                std::ptr::null(),
                creation_disposition,
                windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL
                    | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };

        if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        let associated = unsafe {
            windows_sys::Win32::System::IO::CreateIoCompletionPort(
                handle,
                *iocp.get(),
                0,
                0,
            )
        };
        if associated.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }
            return Err(error);
        }

        Ok(std::sync::Arc::new(tools::ptr::HandleBox::new(handle)))
    }

    fn submit_native_write(
        file: std::sync::Arc<tools::ptr::HandleBox>,
        offset: u64,
        content: bytes::Bytes,
    ) -> std::io::Result<()> {
        let length = u32::try_from(content.len()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "native WriteFile request is larger than u32::MAX",
            )
        })?;

        let mut request = Box::new(NativeWriteRequest {
            overlapped: unsafe { std::mem::zeroed() },
            file,
            content,
            offset,
        });
        request.overlapped.Anonymous.Anonymous.Offset = offset as u32;
        request.overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;

        let requestptr = Box::into_raw(request);
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::WriteFile(
                *(*requestptr).file.get(),
                (*requestptr).content.as_ptr(),
                length,
                std::ptr::null_mut(),
                &mut (*requestptr).overlapped,
            )
        };

        if result == windows_sys::Win32::Foundation::FALSE {
            let error_code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if error_code != windows_sys::Win32::Foundation::ERROR_IO_PENDING {
                unsafe {
                    drop(Box::from_raw(requestptr));
                }
                return Err(std::io::Error::from_raw_os_error(error_code as i32));
            }
        }

        Ok(())
    }

    fn native_iocp_completion_loop(iocp: std::sync::Arc<tools::ptr::HandleBox>) {
        loop {
            let mut transferred = 0u32;
            let mut completion_key = 0usize;
            let mut overlapped = std::ptr::null_mut();

            let result = unsafe {
                windows_sys::Win32::System::IO::GetQueuedCompletionStatus(
                    *iocp.get(),
                    &mut transferred,
                    &mut completion_key,
                    &mut overlapped,
                    u32::MAX,
                )
            };

            if overlapped.is_null() {
                log::error!(
                    "native GetQueuedCompletionStatus returned without OVERLAPPED: {:?}",
                    std::io::Error::last_os_error()
                );
                continue;
            }

            let request = unsafe { Box::from_raw(overlapped as *mut NativeWriteRequest) };
            if result == windows_sys::Win32::Foundation::FALSE {
                log::error!(
                    "native WriteFile completion failed at offset {}: {:?}",
                    request.offset,
                    std::io::Error::last_os_error()
                );
            } else if transferred as usize != request.content.len() {
                log::error!(
                    "native WriteFile short completion at offset {}: {} / {} bytes",
                    request.offset,
                    transferred,
                    request.content.len()
                );
            }
            let filearc = request.file.clone();
            drop(request);

            if let Ok(handle) = std::sync::Arc::try_unwrap(filearc) {
                unsafe {
                    windows_sys::Win32::Foundation::CloseHandle(*handle.get());
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn send_grpc_message_test_test() {

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        let runtime_handle = std::sync::Arc::new(runtime.handle().clone());
        let mut sender = crate::communicate::packager::Sender::new("127.0.0.1", Some(&runtime_handle)).await;
        runtime.spawn(async move {
            sender.dist(crate::communicate::packager::SenderType::Command(crate::communicate::packager::CommandArgs {})).await;
        }).await.unwrap();
    }
}
