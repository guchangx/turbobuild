
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
            SenderType::Archive(args) => {
                
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
    
    async fn pack_windows_kits(&mut self, kits: &str) {
        
    }
    
    pub fn sync_dir(&self, dir: &str, name: &str, compress: bool) -> crate::compiler::compiler::SyncData {
        if compress {
            let response = self.zip_dir(dir, name, client);
            return response;
        }
        else {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                self.sync_dir(entry.path().to_str().unwrap(), name, client, compress);
                            }
                            else if file_type.is_file() {
                                self.sync_file(entry.path().to_str().unwrap());
                            }
                        } 
                        else {
        
                        }
                    }
                }
            }
            return crate::compiler::compiler::SyncData::default();
        }
    }
    
    async fn pack_toolchain(&mut self, path: &str) {
        
        println!("path: {:?}", path);
        let mut path = std::path::PathBuf::from(path);
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\
        //msvc bin dir
        let mut response = self.sync_dir(path.to_str().unwrap(), "msvc", self.client, true);
        let toolchain_bin_path = response.toolchain_path.clone();

        //msvc include
        for _ in 0..3 {
            path.pop();
        }
        let include = path.join("include");

        if include.is_dir() {
            println!("sync dir: {:?}", path);
            response = self.sync_dir(include.to_str().unwrap(), "msvc", self.client, true);
        }
        let toolchain_include_path = response.toolchain_path.clone();
        return (toolchain_bin_path, toolchain_include_path);
    }

    
}