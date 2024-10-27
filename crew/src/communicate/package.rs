use pack::CheckResource;
use winapi::shared::{evntrace, winerror::NOERROR};


pub mod pack {
    include!("../../proto/pack.rs");
}

#[derive(Clone)]
pub struct FileSender {
    client: pack::communicate_client::CommunicateClient<tonic::transport::Channel>,
}

pub struct CommandArgs {

}

pub struct FileArgs {
    
}

pub struct PrecompiledFile<'a> {
    pub name: String,
    pub path: String,
    pub command: String,
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
            client
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
                
            },
            _ => {}
        }
    }
    
    async fn send_cammand(&mut self) {

        let request = tonic::Request::new(pack::CommandTrRequest {
            command: "hello".to_string(),
        });
        
        let response = self.to_owned().client.transmit_command(request).await;
        match response {
            Ok(response) => {
                let inner = response.into_inner();
                if inner.error_code == 0 {
                    println!("send packfile success: {}", inner.error_message);
               }
            }
            Err(err) => {
                println!("send command failed {:?}", err);
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
                    println!("send packfile success: {}", inner.error_message);
                }
            }
            Err(err) => {
                println!("send file failed {:?}", err);
            }
        }
    }

    pub async fn check_resource(&mut self) -> Option<crate::platform::windows::WindowsCompilerEnv> {
        let request = tonic::Request::new(pack::CheckResource {});
        
        let response = self.to_owned().client.check_cocrew_resource(request).await;
        match response {
            Ok(response) => {
                let inner = response.into_inner();
                if inner.error_code == 0 {

                    println!("check cocrew resource success: {:?}", inner);
                    let env = crate::platform::windows::WindowsCompilerEnv {
                        winkits_includes_path: inner.winkits_includes_path.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                        compiler_path: std::path::PathBuf::from(inner.compiler_path),
                        msvc_includes_path: std::path::PathBuf::from(inner.msvc_includes_path),
                        msvc_version: inner.msvc_version,
                        env_args: "".to_string(),
                    };
                    
                    return Some(env);
                }
            }
            Err(err) => {
                println!("check cocrew resource failed {:?}", err);
            }
        }

        return None;
    }
}