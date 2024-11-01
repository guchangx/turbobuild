
use std::io::Write;

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
            
            let addr = "0.0.0.0:19302".parse().expect("parse addr failed");
            log::debug!("init cocrew communicate server {}", addr);
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
                    log::debug!("run communicate rpc service end");
                },
                Err(err) => {
                    panic!("run communicate rpc service failed: {}", err);
                }
            }
        });

    }

    async fn transmit_file_handle(&self, request: package::FileTrRequest) -> package::FileTrResponse {

        let name = request.name;
        let path = request.path;
        
        log::debug!("transmit file handle name: {}, path: {}", name, path);

        let content = request.content;
        let file_type = request.file_type;
        
        if file_type == package::FileType::Toolchain as i32 {
                Self::extract(&path, &content).await;
        }
            
        else if file_type == package::FileType::Kits as i32 {
                
        }
        else {
            log::debug!("unknown file type: {}", file_type);
        }
        
        let reply = package::FileTrResponse {
            error_code: 0,
            error_message: "sync file success.".to_string(),
        };
        
        return reply;
    }
    
    async fn transmit_compile_handle(&self, request: package::CompileTrRequest) -> package::CompileTrResponse {

        let file = request.file;
        let compiler = request.compiler;

        log::trace!("compile handle name: {} {}", file, compiler);
        let commands = request.commands;
        let content = request.content;

        let mut file = std::fs::File::create(&file).unwrap();
        match file.write_all(&content) {
            Ok(_) => {
                log::trace!("sync file done: {:?}", file);
            },
            Err(err) => {
                log::error!("sync file failed. {:?}", err)
            }
        }

        if commands.is_empty() {
        
        }
        else {
            let compiler_input = crew::compiler::model::CompilerInput {
                compiler_path: std::ffi::OsString::from(compiler),
                compiler_working_dir: std::ffi::OsString::from(request.working_dir),
                compiler_commands: commands.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                build_and_compiler_type: std::ffi::OsString::from(request.variety),
            };

            crate::compiler::interface::build(compiler_input);
        }

        let reply = package::CompileTrResponse {
            error_code: 0,
            error_message: "sync compile success.".to_string(),
        };
        
        return reply;
    }

    async fn persistence(path: &str, content: &[u8]) {
        
    }

    async fn extract(path: &str, content: &[u8]) {
        if path.ends_with(".zip") {
            let cursor = std::io::Cursor::new(content);
            let mut zip = zip::ZipArchive::new(cursor).unwrap();
            let replica = crew::replica::toolchain::Property::new("".to_string(), path.to_string());
            let replica_path = replica.access_replica_toolchain_path();
            zip.extract(replica_path).unwrap();
        }
        else {
            
        }
    }
}

#[tonic::async_trait]
impl package::communicate_server::Communicate for FileReceiver {
    async fn transmit_file(&self, request: tonic::Request<package::FileTrRequest>) -> core::result::Result<tonic::Response<package::FileTrResponse>, tonic::Status> {
        log::debug!("sync request transmit file.");
        
        let tr_file = request.into_inner();
        let reply = self.transmit_file_handle(tr_file).await;

        Ok(tonic::Response::new(reply))
    }
    
    async fn transmit_compile(&self, request: tonic::Request<package::CompileTrRequest>) -> core::result::Result<tonic::Response<package::CompileTrResponse>, tonic::Status> {
        
        let rt_compile = request.into_inner();
        log::debug!("sync request command: {:?} {:?}", rt_compile.compiler, rt_compile.commands);
        
        let reply = self.transmit_compile_handle(rt_compile).await;
        Ok(tonic::Response::new(reply))
    }
}


