use std::io::Write;
use tokio_stream::StreamExt;


pub mod pack {
    include!("../../proto/pack.rs");
}

#[derive(Clone)]
pub struct FileSender {
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
    ToolChain = 1,
    Kits = 2,
}

pub struct ArchiveArgs<'a> {
    pub file_type: FileType,
    pub name: String,
    pub path: String,    
    pub content: std::borrow::Cow<'a, [u8]>,
}

pub enum SenderType<'a> {
    Command(CommandArgs),
    Archive(ArchiveArgs<'a>),
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

impl FileSender {
    pub fn new(addr: &str, runtime: Option<&std::sync::Arc<tokio::runtime::Handle>>) -> Self {
        let mut host = "localhost"; 
        if !addr.is_empty() {
            host = addr;
        }
        
        let channel = tonic::transport::Endpoint::from_shared(std::format!("http://{}:19302", host)).unwrap()
            .connect_lazy();

        let client = pack::communicate_client::CommunicateClient::new(channel)
            .max_decoding_message_size(1024 * 1024 *60)
            .max_encoding_message_size(1024 * 1024 *60);

        let sender = FileSender {
            client,
            host: host.to_string(),
            runtime: runtime.cloned(),
        };

        return sender;
    }
    
    pub async fn send<'a>(&mut self, sender_type: SenderType<'a>) -> ReceiverType {
        
        match sender_type {
            SenderType::Command(_args) => {
                let result = CommandRecv {
                    status: true,
                    message: "".to_string(),
                };
                return ReceiverType::Command(result);
            },
            SenderType::Archive(args) => {
                self.dist_file(args).await;
                let result = ArchiveRecv {
                    status: false,
                    message: "".to_string(),
                };
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
    
    async fn dist_file(&mut self, args: ArchiveArgs<'_>) {

        let request = tonic::Request::new(pack::FileTrRequest {
            file_type: args.file_type as i32,
            name: args.name,
            path:  args.path,
            content: args.content.to_vec(),
        });
        
        let response = self.to_owned().client.transmit_file(request).await;
        match response {
            Ok(response) => {
                let inner = response.into_inner();
                if inner.error_code == 0 {
                    log::debug!("send to {} packfile success. response: {}", self.host ,inner.error_message);
                }
            }
            Err(err) => {
                log::error!("send file to {}:19302, failed: {:?}", self.host, err);
            }
        }
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

                                }
                                else if response.progress == pack::CompileProgress::Compiledone as i32 {
                                    log::debug!("precompiled sourcefile done response: {}", response.tips);

                                    recv.status = response.status;
                                    recv.out = response.out;
                                    recv.err = response.err;

                                    let mut myself = self.clone();
                                    self.runtime.clone().unwrap().spawn(async move {
                                        myself.save_compile_output(&response.results).await;
                                    });
                                }
                                else {
                                    
                                }
                            }
                            else {
                                log::warn!("send precompiled sourcefile reveice response failed. {}", response.tips);
                            }
                        },
                        Err(err) => {
                            log::error!("send precompiled sourcefile receive response failed. {}", err);
                            break;
                        },
                    }
                }
                log::info!("send precompiled sourcefile receive response done.");
            }
            Err(err) => {
                log::warn!("send precompiled sourcefile failed {:?}", err);
            }
        }
        return recv;
    }

    async fn save_compile_output(&mut self, results: &Vec<crate::communicate::package::pack::IntermediateResult>) {
        for result in results {
            log::debug!("save compile result: {:?}", result.file);

            match std::fs::OpenOptions::new().read(true).write(true).create(true).open(&result.file) {
                Ok(file) => {
                    let mut writer = std::io::BufWriter::new(file);
                    writer.write_all(&result.content).unwrap();
                    writer.flush().unwrap();
                },
                Err(err) => {
                    log::error!("create file failed: {}, path: {}", err, result.file);
                }
            }
        }
    }
}