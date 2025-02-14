use std::io::Write;
use tokio_stream::StreamExt;


pub mod pack {
    include!("../../proto/pack.rs");
}

#[derive(Clone)]
pub struct FileSender {
    client: pack::communicate_client::CommunicateClient<tonic::transport::Channel>,
    host: String,
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
    pub status: bool,
    pub message: String,
}

pub enum ReceiverType {
    Command(CommandRecv),
    Archive(ArchiveRecv),
    Compile(CompileRecv),
    None,
}

impl FileSender {
    pub fn new(addr: &str) -> Self {
        let mut host = "localhost"; 
        if !addr.is_empty() {
            host = addr;
        }
        
        let channel = tonic::transport::Endpoint::from_shared(std::format!("http://{}:19302", host)).unwrap()
            .connect_lazy();

        let client = pack::communicate_client::CommunicateClient::new(channel);
        
        let sender = FileSender {
            client,
            host: host.to_string(),
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
                self.send_file(args).await;
                let result = ArchiveRecv {
                    status: false,
                    message: "".to_string(),
                };
                return ReceiverType::Archive(result);
            },
            SenderType::Compile(args) => {
                self.send_compile(args).await;

                let result = CompileRecv {
                    status: true,
                    message: "".to_string(),
                };
                return ReceiverType::Compile(result);
            },
            _ => {
                return ReceiverType::None;
            }
        }
    }
    
    async fn send_file(&mut self, args: ArchiveArgs<'_>) {

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
                log::error!("send file failed {:?}", err);
            }
        }
    }

    async fn send_compile(&mut self, compiled: PrecompiledFile<'_>) {
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

        match response {
            Ok(response) => {
                let mut stream = response.into_inner();
                while let Some(inner) = stream.next().await {
                    match inner {
                        Ok(response) => {
                            if response.error_code == 0 {
                                log::debug!("send precompiled sourcefile response success: {}", response.error_message);

                                if response.progress == pack::CompileProgress::Filetransfer as i32 {

                                }
                                else if response.progress == pack::CompileProgress::Compilestart as i32 {

                                }
                                else if response.progress == pack::CompileProgress::Compiledone as i32 {
                                   log::info!("response info: {:?}", response.info);
                                   //TODO should use async runtime
                                   
                                   self.save_compile_result(&response.results).await;

                                }
                                else {
                                    
                                }
                            }
                            else {
                                log::warn!("send precompiled sourcefile response failure: {}", response.error_message);
                            }
                        },
                        Err(err) => {
                            log::error!("send compiled response failed. {}", err);
                        },
                    }
                }
            }
            Err(err) => {
                log::warn!("send compiled failed {:?}", err);
            }
        }
    }

    async fn save_compile_result(&mut self, results: &Vec<crate::communicate::package::pack::IntermediateResult>) {
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