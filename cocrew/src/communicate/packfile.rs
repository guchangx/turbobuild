
pub mod package {
    include!("../../proto/packfile.rs");
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
}

#[tonic::async_trait]
impl package::communicate_server::Communicate for FileReceiver {
    async fn transmit(&self, request: tonic::Request<package::Request>) -> core::result::Result<tonic::Response<package::Response>, tonic::Status> {
        println!("sync request: {:?}", request);
        
        let reply = package::Response {
            message: "sync success".to_string(),
            error_code: 0,
            error_message: "".to_string(),
        };
        
        Ok(tonic::Response::new(reply))
    }
}


