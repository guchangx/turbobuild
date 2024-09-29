
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

pub struct ArchiveArgs {
    pub name: String,
    pub path: String,    
    pub content: std::io::Cursor<Vec<u8>>
}

pub enum SenderType {
    Command(CommandArgs),
    File(FileArgs),
    Archive(ArchiveArgs),
} 

impl FileSender {
    pub fn new() -> Self {
        let channel = tonic::transport::Endpoint::from_shared("http://localhost:19302").unwrap()
            .connect_lazy()
            .unwrap();

        let mut client = pack::communicate_client::CommunicateClient::new(channel);
        
        let sender = FileSender {
            client
        };
        return sender;
    }
    
    pub async fn send(&mut self, sender_type: SenderType) {
        
        match sender_type {
            SenderType::Command(args) => {
                
            },
            SenderType::File(args) => {

            },
            SenderType::Archive(args    ) => {
                
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
    async fn send_file(&mut self) {

        let request = tonic::Request::new(pack::FileTrRequest {
            path: "hello".to_string(),
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