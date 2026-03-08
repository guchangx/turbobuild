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
    pub solution: String,
    pub project: String,
    pub name: String,
    pub path: String,
    pub content: std::borrow::Cow<'a, [u8]>,
}

pub struct ArchiveStreamArgs {
    pub rx: tokio::sync::mpsc::Receiver<super::packager::ArchiveArgs<'static>>,
    pub callback: Box<dyn Fn() + Send + Sync>,
}

pub enum SenderType<'a> {
    Command(CommandArgs),
    Archive(ArchiveArgs<'a>),
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

impl Sender {
    pub async fn new(addr: &str, runtime: Option<&std::sync::Arc<tokio::runtime::Handle>>) -> Self {
        let mut host = "localhost"; 
        if !addr.is_empty() {
            host = addr;
        }
        
        let channel = tonic::transport::Endpoint::from_shared(std::format!("http://{}:19302", host)).unwrap()
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
    
    async fn dist_archive(&mut self, args: ArchiveArgs<'_>) -> ArchiveRecv {

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
        let request = pack::FileTrRequest {
            file_type: args.file_type as i32,
            solution: args.solution.clone(),
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
                    solution: archive.solution.clone(),
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
                            //log::debug!("transmit file {} response code: {}, message: {}", host, stream.error_code, stream.error_message);
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
            commands: compile.commands,
            envs: compile.envs.iter().map(|(k,v)| pack::Envs {
                key: k.clone(),
                value: v.clone(),
            }).collect(),
            
            content: compile.content.to_vec(),
        });

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
                                    log::debug!("compiled sourcefile start response: {}", response.tips);
                                }
                                else if response.progress == pack::CompileProgress::Compiling as i32 {

                                    log::trace!("compiling receive compiled sourcefile response: {:?}", response.results.iter().map(|item| item.file.clone()).collect::<Vec<_>>());
                                    
                                    tx.send(response.results).await.unwrap_or_else(|err| {
                                        log::error!("send compiled sourcefile response to save failed: {:?}", err);
                                    });

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
            
                                    log::debug!("compiled sourcefile done response out: {:?}", String::from_utf8_lossy(&response.out));
                                    recv.out = response.out;
                                
                                    log::debug!("compiled sourcefile done response err: {:?}", String::from_utf8_lossy(&response.err));
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
                                log::debug!("compiled sourcefile failed response out: {:?}", String::from_utf8_lossy(&response.out));
                                log::debug!("compiled sourcefile failed response err: {:?}", String::from_utf8_lossy(&response.err));
                                recv.status = response.status;
                                recv.out = response.out;
                                recv.err = response.err;
                                break;
                            }
                        },
                        Err(err) => {
                            log::error!("send compiled sourcefile receive response failed. {}", err);
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
        log::info!("{} save compile output from channel done.", project);
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