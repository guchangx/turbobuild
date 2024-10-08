
pub mod package {
    include!("../../proto/pack.rs");
}

#[derive(Default, Clone)] 
pub struct FileReceiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
}

impl FileReceiver {
    
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        
        let receiver = FileReceiver {
            common,
        };
        return receiver;
    }

    pub fn init(&self) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            println!("init communicate server");
            let addr = "127.0.0.1:19302".parse().expect("parse addr failed");
            
            let receiver = FileReceiver {
                common: self.common.clone(),
            };
            
            let server = package::communicate_server::CommunicateServer::new(receiver);
    
            let result = tonic::transport::Server::builder()
                .add_service(server)
                .serve(addr)
                .await;
            
            match result {
                Ok(_) => {
                    println!("run communicate rpc service end");
                },
                Err(err) => {
                    panic!("run communicate rpc service failed: {}", err);
                }
            }
        });

    }

    async fn transmit_file(&self, request: package::FileTrRequest) -> package::FileTrResponse {

        println!("sync request: {:?}", request);
        let name = request.name;
        let path = request.path;
        let content = request.content;
        let file_type = request.file_type;
        
        if file_type == package::FileType::Toolchain as i32 {
                Self::extract(&path, &content).await;
            }
            
        else if file_type == package::FileType::Kits as i32 {
                
        }
        else {
            println!("unknown file type: {}", file_type);
                        
        }
        
        let reply = package::FileTrResponse {
            error_code: 0,
            error_message: "sync file success.".to_string(),
        };
        
        return reply;
    }
    
    async fn persistence(path: &str, content: &[u8]) {
        
    }

    async fn extract(path: &str, content: &[u8]) {
        if path.ends_with(".zip") {
            let cursor = std::io::Cursor::new(content);
            let mut zip_archive = zip::ZipArchive::new(cursor).unwrap();
            let replica = crate::replica::toolchain::Property::new("".to_string(), path.to_string());
            let replica_path = replica.fetch_replica_path();
            zip_archive.extract(replica_path).unwrap();  
        }
        else {
            
        }
    }
}

#[tonic::async_trait]
impl package::communicate_server::Communicate for FileReceiver {
    async fn transmit_file(&self, request: tonic::Request<package::FileTrRequest>) -> core::result::Result<tonic::Response<package::FileTrResponse>, tonic::Status> {
        println!("sync request: {:?}", request);
        
        let tr_file = request.into_inner();
        let reply = self.transmit_file(tr_file).await;
        
        Ok(tonic::Response::new(reply))
    }
    
    async fn transmit_command(&self, request: tonic::Request<package::CommandTrRequest>) -> core::result::Result<tonic::Response<package::CommandTrResponse>, tonic::Status> {
        println!("sync request: {:?}", request);
        
        let reply = package::CommandTrResponse {
            error_code: 0,
            error_message: "sync command success.".to_string(),
        };
        
        Ok(tonic::Response::new(reply))
    }
}


