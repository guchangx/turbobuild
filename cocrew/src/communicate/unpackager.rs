
use std::io::Write;

#[allow(non_camel_case_types)]
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

    pub async fn init(&self) {   
        let addr = "0.0.0.0:19302".parse().expect("parse addr failed");
        log::debug!("init cocrew communicate server {}", addr);

        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.set_reuseaddr(true).unwrap();

        socket.bind(addr).unwrap();
        let listener = socket.listen(1024).unwrap();

        let receiver = FileReceiver {
            common: self.common.clone(),
        };

        let server = package::communicate_server::CommunicateServer::new(receiver);
    
        let result = tonic::transport::Server::builder()
            .tcp_nodelay(true)
            .add_service(server.max_decoding_message_size(1024 *1024 *60).max_encoding_message_size(1024 * 1024 *60))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            //.serve(addr)
            .await;
        
        match result {
            Ok(_) => {
                log::debug!("run communicate rpc service end");
            },
            Err(err) => {
                panic!("run communicate rpc service failed. addr {:?},  {:?}", addr, err);
            }
        }
    }

    async fn transmit_file_handle(&self, request: package::FileTrRequest) -> package::FileTrResponse {

        let name = request.name;
        let path = request.path;
        
        log::debug!("transmit file handle name: {}, path: {}", name, path);

        let content = request.content;
        let file_type = request.file_type;
        
        if file_type == package::FileType::Toolchain as i32 {
            if path.ends_with(".zip") {
                let project = crew::replica::toolchain::Property::new(&path[..(path.len() - ".zip".len())]);
                let path = project.access_replica_toolchain_path();
    
                Self::extract(&path, &content).await;
            }
            else {
                log::error!("package toolchain is not end with .zip {}", path);
            }
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
    
    async fn transmit_task_handle(&self, request: package::CompileTrRequest, tx: tokio::sync::mpsc::Sender<Result<package::CompileTrResponse, tonic::Status>>) {

        let project = request.project;
        let file = request.file;
        let compiler = request.compiler;
    
        let commands = request.commands;
        let content = request.content;
        log::trace!("transmit compile handle project: {}, file: {}, compiler: {}, commands is empty: {}, content size: {}KB.", project, file, compiler, commands.is_empty(), &content.len() / 1024 );

        if file.is_empty() {

            let compiler_input = crew::compiler::model::CompilerInput {
                project: std::ffi::OsString::from(project),
                compiler_path: std::ffi::OsString::from(compiler),
                compiler_working_dir: std::ffi::OsString::from(request.working_dir),
                compiler_commands: commands.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                build_and_compiler_type: std::ffi::OsString::from(request.variety),
            };

            let reply = Self::execute(&compiler_input).await;
            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
        }
        else if !content.is_empty() {

            let reply = Self::storage(&project, &file, &content).await;
            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
            
        }
        else {
            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Filetransfer.into(),
                info: "".to_string(),
                results: Vec::new(),
                error_code: 0,
                error_message: "sync compile success.".to_string(),
            };
            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
        }
    }

    async fn storage(project: &str, path: &str, content: &[u8]) -> package::CompileTrResponse {

        let mut reply = package::CompileTrResponse {
            progress: package::CompileProgress::Filetransfer.into(),
            info: "".to_string(),
            results: Vec::new(),
            error_code: 0,
            error_message: "transmit do save file success.".to_string(),
        };

        if project.is_empty() || path.is_empty() {
            reply.error_message = "project or path param is empty, so do nothing".to_string();
            log::error!("transmit storage project name or path is empty.");
        }
        else {
            let project = crew::replica::project::Property::new( project, path);
            let path = project.fetch_local_replica_project_path();
            
            log::trace!("replica storage path: {:?}", path);
    
            if path.extension() == Some(&std::ffi::OsStr::new("zip")) {
                Self::extract(&path.to_str().unwrap(), &content).await;
            }
            else {
                
                let mut file = std::fs::File::create(&path).unwrap();
                match file.write_all(&content) {
                    Ok(_) => {
                        log::trace!("transmit storage file done: {:?}", path);
            
                    },
                    Err(err) => {
                        log::error!("transmit storage file failed. {:?} {:?}", path, err)
                    }
                }
            }
        }
        
        return reply;   
    }

    async fn extract(path: &str, content: &[u8]) {
        if path.ends_with(".zip") {
            let dir = &path[..(path.len() - ".zip".len())];
            let cursor = std::io::Cursor::new(content);
            let mut zip = zip::ZipArchive::new(cursor).unwrap();
            match zip.extract(dir) {
                Ok(_) => {
                    log::trace!("extract zip file done: {}", dir);
                },
                Err(err) => {
                    log::error!("extract zip file failed. {} {}", dir, err);
                }
            }
        }
        else {
            let cursor = std::io::Cursor::new(content);
            let mut zip = zip::ZipArchive::new(cursor).unwrap();
            log::trace!("extract zip file names {:?}", zip.file_names().collect::<Vec<&str>>());
            match zip.extract(path) {
                Ok(_) => {
                    log::trace!("extract zip file done: {}", path);
                },
                Err(err) => {
                    log::error!("extract zip file failed. {} {}", path, err);
                }
            }
        }
    }
    async fn execute(input: &crew::compiler::model::CompilerInput) -> package::CompileTrResponse {
        let (output, results) = crate::compiler::interface::build(input.to_owned());
        if output.status {
            log::info!("compile successed filename {:?} status {:?}", output.filename, true);
            let mut intermediates = Vec::new();
            if let Some(results) = results {
                for result in results {
                    let source = result.source_file;
      
                    if let Some(obj)  = result.obj {
                        let file = obj.0;
                        let content = obj.1;
                        let intermediate = package::IntermediateResult {
                            file: file.to_string_lossy().into(),
                            content: content,
                        };
                        intermediates.push(intermediate);
                    }
                    else if let Some(idb) = result.idb {
                        let file = idb.0;
                        let content = idb.1;
                        let intermediate = package::IntermediateResult {
                            file: file.to_string_lossy().into(),
                            content: content,
                        };
                        intermediates.push(intermediate);
                    }
                    else if let Some(pdb) = result.pdb {
                        let file = pdb.0;
                        let content = pdb.1;
                        let intermediate = package::IntermediateResult {
                            file: file.to_string_lossy().into(),
                            content: content,
                        };
                        intermediates.push(intermediate);
                    }
                }
            };
            
            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Compiledone.into(),
                info: output.filename.iter().map(|item| item.to_string_lossy()).collect(),
                results: intermediates,
                error_code: 0,
                error_message: "transmit do compile success.".to_string(),
            };
            return reply;
        }
        else {
            log::info!("compile failed. filename {:?} status {:?}", output.filename, false);
            
            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Compiledone.into(),
                info: output.output.to_string_lossy().to_string(),
                results: Vec::new(),
                error_code: 0,
                error_message: "transmit do compile failed.".to_string(),
            };
            return reply;
        }
    }
    
}

type ResponseStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::CompileTrResponse, tonic::Status>> + Send>>;



#[tonic::async_trait]
impl package::communicate_server::Communicate for FileReceiver {
    async fn transmit_file(&self, request: tonic::Request<package::FileTrRequest>) -> core::result::Result<tonic::Response<package::FileTrResponse>, tonic::Status> {
        log::debug!("sync request transmit file.");
        
        let tr_file = request.into_inner();
        let reply = self.transmit_file_handle(tr_file).await;

        Ok(tonic::Response::new(reply))
    }
    
    type transmit_taskStream = ResponseStream;
    async fn transmit_task(&self, request: tonic::Request<package::CompileTrRequest>) -> core::result::Result<tonic::Response<Self::transmit_taskStream>, tonic::Status> {
        
        let rt_compile = request.into_inner();
        log::debug!("sync request transmit compile: {:?} \n commands: {:?}", rt_compile.compiler, rt_compile.commands);
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let self_ = self.clone();
        let _ = tokio::task::spawn(async move {
            self_.transmit_task_handle(rt_compile, tx).await;
        }).await;

        let response = tokio_stream::wrappers::ReceiverStream::new(rx);
        return Ok(tonic::Response::new(Box::pin(response) as ResponseStream));
    }
}