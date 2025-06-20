
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;


pub mod pack {
    include!("../../proto/pack.rs");
}

#[derive(Clone)]
pub struct Sender {
    client: pack::communicate_client::CommunicateClient<tonic::transport::Channel>,
    host: String,
    runtime: Option<std::sync::Arc<tokio::runtime::Handle>>,
}

pub struct CommandArgs {

}

pub struct FileArgs {
    
}

pub struct PrecompiledFile<'a> {
    pub project: String,
    pub file: String,
    pub compiler: String,
    pub working_dir: String,
    pub variety: String,
    pub commands: Vec<String>,
    pub content: std::borrow::Cow<'a, [u8]>,
}

pub enum FileType {
    Unknown = 0,
    SourceFiles = 1,
    PrecompiledSrcFiles = 2,
    ToolChain = 3,
    Kits = 4,
}

pub struct ArchiveArgs<'a> {
    pub file_type: FileType,
    pub project: String,
    pub name: String,
    pub path: String,    
    pub content: std::borrow::Cow<'a, [u8]>,
}

pub struct ArchiveStreamArgs {
    pub rx: tokio::sync::mpsc::Receiver<super::package::ArchiveArgs<'static>>,
    pub callback: Box<dyn Fn() + Send + Sync>,
}

pub enum SenderType<'a> {
    Command(CommandArgs),
    Archive(ArchiveArgs<'a>),
    ArchiveStream(ArchiveStreamArgs),
    Compile(PrecompiledFile<'a>),
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

impl Sender {
    pub fn new(addr: &str, runtime: Option<&std::sync::Arc<tokio::runtime::Handle>>) -> Self {
        let mut host = "localhost"; 
        if !addr.is_empty() {
            host = addr;
        }
        
        let channel = tonic::transport::Endpoint::from_shared(std::format!("http://{}:19302", host)).unwrap()
            .connect_timeout(std::time::Duration::from_secs(15))
            .connect_lazy();

        let client = pack::communicate_client::CommunicateClient::new(channel)
            .max_decoding_message_size(1024 * 1024 * 180 * 2)
            .max_encoding_message_size(1024 * 1024 * 180 * 2);
        let sender = Sender {
            client,
            host: host.to_string(),
            runtime: runtime.cloned(),
        };

        return sender;
    }
    
    pub async fn dist<'a>(&mut self, sender_type: SenderType<'a>) -> ReceiverType {
        
        match sender_type {
            SenderType::Command(_args) => {
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
            SenderType::Compile(args) => {
                let result = self.dist_compile(args).await;
                return ReceiverType::Compile(result);
            },
            _ => {
                return ReceiverType::None;
            }
        }
    }
    
    async fn dist_archive(&mut self, args: ArchiveArgs<'_>) -> ArchiveRecv {

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
        let request = pack::FileTrRequest {
            file_type: args.file_type as i32,
            project: args.project.clone(),
            name: args.name,
            path:  args.path,
            content: args.content.to_vec(),
        };

        if let Err(err) = tx.send(request).await {
            log::error!("transmit file error: {:?}", err);
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
                    project: archive.project.clone(),
                    name: archive.name,
                    path: archive.path,
                    content: archive.content.to_vec(),
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
                            log::debug!("transmit file {} response code: {}, message: {}", host, stream.error_code, stream.error_message);

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
                log::error!("transmit file  {} failed: {:?}", self.host, err);
                result.status = false;
            }
        };

        if let Some(handle) = handle {
            let _ = handle.await.unwrap();
        }

        return result;
    }

    //TODO should think split dist compiler command or ziped precompilre sourcefile.
    async fn dist_compile(&mut self, compiled: PrecompiledFile<'_>) -> CompileRecv {
        let request = tonic::Request::new(pack::CompileTrRequest {
            project: compiled.project,
            file: compiled.file,
            compiler: compiled.compiler,
            working_dir: compiled.working_dir,
            variety: compiled.variety,
            commands: compiled.commands,
            content: compiled.content.to_vec(),
        });

        let response = self.to_owned().client.transmit_task(request).await;

        let mut recv = CompileRecv {
            status: 0,
            out: Vec::new(),
            err: Vec::new(),
        };

        match response {
            Ok(response) => {
                let mut stream = response.into_inner();
                while let Some(inner) = stream.next().await {
                    match inner {
                        Ok(response) => {
                            if response.status == 0 {
                                if response.progress == pack::CompileProgress::Filetransfer as i32 {

                                }
                                else if response.progress == pack::CompileProgress::Compilestart as i32 {
                                    log::debug!("precompiled sourcefile start response: {}", response.tips);
                                }
                                else if response.progress == pack::CompileProgress::Compiling as i32 {
                                    let mut myself = self.clone();

                                    log::trace!("compiling receive precompiled sourcefile response: {:?}", response.results.iter().map(|item| item.file.clone()).collect::<Vec<_>>());

                                    let runtime = myself.runtime.clone().unwrap();
                                    myself.save_compile_output(&response.results, &runtime).await;

                                }
                                else if response.progress == pack::CompileProgress::Compiledone as i32 {
            
                                    log::debug!("precompiled sourcefile done response: {:?}", String::from_utf8_lossy(&response.out));
                                    recv.out = response.out;
                                
                                    log::debug!("precompiled sourcefile done response: {:?}", String::from_utf8_lossy(&response.err));
                                    recv.err = response.err;
                                    
                                    recv.status = response.status;
                                    
                                    let mut myself = self.clone();
                                    self.runtime.clone().unwrap().spawn(async move {
                                        let runtime = myself.runtime.clone().unwrap();
                                        myself.save_compile_output(&response.results, &runtime).await;
                                    });
                                    
                                }
                                else {
                                    
                                }
                            }
                            else {
                                log::warn!("send precompiled sourcefile reveice response failed. {}", response.tips);
                                recv.status = response.status;
                                recv.out = response.out;
                                recv.err = response.err;
                            }
                        },
                        Err(err) => {
                            log::error!("send precompiled sourcefile receive response failed. {}", err);
                            recv.status = 1;
                            recv.err = err.to_string().into_bytes();
                            break;
                        },
                    }
                }
                log::info!("send precompiled sourcefile receive response done.");
            }
            Err(err) => {
                log::warn!("send precompiled sourcefile failed: {:?}", err);
                recv.status = 1;
                recv.err = err.to_string().into_bytes();
            }
        }
        return recv;
    }

    async fn save_compile_output(&mut self, results: &[crate::communicate::package::pack::IntermediateResult], runtime: &std::sync::Arc<tokio::runtime::Handle>) {
        let mut handles = Vec::new();
        for result in results.to_owned() {

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
        log::info!("save compile output done. file count: {}", results.len());
    }
}