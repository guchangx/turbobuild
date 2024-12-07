use pack::CheckResource;
use tokio_stream::StreamExt;
use winapi::shared::{evntrace, winerror::NOERROR};


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
    
    pub async fn send<'a>(&mut self, sender_type: SenderType<'a>) {
        
        match sender_type {
            SenderType::Command(_args) => {
                
            },
            SenderType::Archive(args) => {
                self.send_file(args).await;
            },
            SenderType::Compile(args) => {
                self.send_compile(args).await;
            },
            _ => {}
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

        let response = self.to_owned().client.transmit_compile(request).await;

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

                                }
                                else {
                                    
                                }
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
}