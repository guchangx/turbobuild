
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
} 

impl FileSender {
    pub fn new() -> Self {
        let channel = tonic::transport::Endpoint::from_shared("http://localhost:19302").unwrap()
            .connect_lazy();

        let mut client = pack::communicate_client::CommunicateClient::new(channel);
        
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
            }
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
}