
use std::io::Write;

#[allow(non_camel_case_types)]
pub mod package {
    include!("../../proto/pack.rs");
}

#[derive(Default, Clone)] 
pub struct Receiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
}

impl Receiver {
    
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        
        let receiver = Receiver {
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

        let receiver = Receiver {
            common: self.common.clone(),
        };

        let server = package::communicate_server::CommunicateServer::new(receiver);
    
        let result = tonic::transport::Server::builder()
            .tcp_nodelay(true)
            .add_service(server.max_decoding_message_size(1024 *1024 * 180 * 2).max_encoding_message_size(1024 * 1024 * 180 * 2))
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
                //TODO: report resource again to captain. 

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

            let handle = Self::check_dir_exists(&project, &commands).await;

            let compiler_input = crew::compiler::model::CompilerInput {
                project: std::ffi::OsString::from(project),
                compiler_path: std::ffi::OsString::from(compiler),
                compiler_working_dir: std::ffi::OsString::from(request.working_dir),
                compiler_commands: commands.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                build_and_compiler_type: std::ffi::OsString::from(request.variety),
            };
            let _ = handle.await;
            
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
                out: Vec::new(),
                err: Vec::new(),
                results: Vec::new(),
                status: 0,
                tips: "sync compile success.".to_string(),
            };
            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
        }
    }

    async fn storage(project: &str, path: &str, content: &[u8]) -> package::CompileTrResponse {
        let now = std::time::Instant::now();
        
        let mut reply = package::CompileTrResponse {
            progress: package::CompileProgress::Filetransfer.into(),
            out: Vec::new(),
            err: Vec::new(),
            results: Vec::new(),
            status: 0,
            tips: "transmit do save file success.".to_string(),
        };

        if project.is_empty() || path.is_empty() {
            reply.tips = "project or path param is empty, so do nothing".to_string();
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

        log::info!("transmit storage file done. elapsed: {:?}", now.elapsed());
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
        let (output, results) = crate::compiler::interface::cocrew_build(input.to_owned());
        if output.status == 0 {
            let mut intermediates = Vec::new();
            if let Some(results) = results {
                for result in results {
                    let _source = result.source_file;
      
                    if let Some(obj)  = result.obj {
                        let file = obj.0;
                        let content = obj.1;
                        let intermediate = package::IntermediateResult {
                            file: file.to_string_lossy().into(),
                            content: content,
                        };
                        intermediates.push(intermediate);
                    }

                    if let Some(idb) = result.idb {
                        let file = idb.0;
                        let content = idb.1;
                        let intermediate = package::IntermediateResult {
                            file: file.to_string_lossy().into(),
                            content: content,
                        };
                        intermediates.push(intermediate);
                    }
                    if let Some(pdb) = result.pdb {
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
                out: output.out.to_vec(),
                err: output.err.to_vec(),
                results: intermediates,
                status: output.status,
                tips: "transmit do compile success.".to_string(),
            };
            return reply;
        }
        else {  
            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Compiledone.into(),
                out: output.out.to_vec(),
                err: output.err.to_vec(),
                results: Vec::new(),
                status: 1,
                tips: "transmit do compile failed.".to_string(),
            };
            return reply;
        }
    }

    //create dir for .pdb, if parent dir not exist, .pdb file can not be generated.
    pub async fn check_dir_exists(project: &String, commands: &Vec<String>) -> tokio::task::JoinHandle<()> {

        let project = project.clone();
        let commands = commands.clone();

        let rt = crate::common::RUNTIME.lock().unwrap();
        let handel = rt.spawn(async move {
            let path = commands.iter().find(|&item| item.starts_with("/Fd")).map(|item| item.clone());

            match path {
                Some(mut path) => {
                    let pdb = path.split_off(3);
                    let pdb = std::path::PathBuf::from(pdb);
                    if pdb.has_root() && pdb.is_absolute() {

                        let replica = std::path::PathBuf::from(tools::utils::access_replica_dir());
    
                        let split = |pdb: std::path::PathBuf, project: &String| {
                            if project.is_empty() {
                                return Some(pdb);
                            }
                            else {
                                let components = pdb.components().collect::<Vec<_>>();
                                if let Some(index) = components.iter().position(|item| item.as_os_str().to_str().unwrap() == project) {
                                    let result: std::path::PathBuf = components[index + 1..].iter().collect();
                                    let path = replica.join("Project").join(project).join(result);
                                    let path = path.parent().unwrap().to_owned();
                                    return Some(path);
                                }
                                else {
                                    log::error!("not find project in path: {:?} project: {}.", pdb, project);
                                    return None;
                                }            
                            }
                        };
                        
                        if let Some(path) = split(pdb, &project) {
                            let path = replica.join("Project").join(project).join(path);
                            if !path.exists() {
                                log::info!("check dir not exist, so need create dir: {:?}", path);
                                let _ = std::fs::create_dir_all(path);
                            }
                        }
                        else {
                        }
                    }
                },
                None => {},
            }
        });
        return handel;
    }
    
}

type ResponseStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::CompileTrResponse, tonic::Status>> + Send>>;


#[tonic::async_trait]
impl package::communicate_server::Communicate for Receiver {
    async fn transmit_file(&self, request: tonic::Request<package::FileTrRequest>) -> core::result::Result<tonic::Response<package::FileTrResponse>, tonic::Status> {
        log::debug!("sync request transmit file. from: {:?}", request.remote_addr());
        
        let tr_file = request.into_inner();
        let reply = self.transmit_file_handle(tr_file).await;

        Ok(tonic::Response::new(reply))
    }
    
    type transmit_taskStream = ResponseStream;
    async fn transmit_task(&self, request: tonic::Request<package::CompileTrRequest>) -> core::result::Result<tonic::Response<Self::transmit_taskStream>, tonic::Status> {
        
        let rt_compile = request.into_inner();
        log::debug!("sync request transmit compiler: {:?}", rt_compile.compiler);
        log::debug!("commands: {:?}", rt_compile.commands);
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let self_ = self.clone();
        let _ = tokio::task::spawn(async move {
            self_.transmit_task_handle(rt_compile, tx).await;
        }).await;

        let response = tokio_stream::wrappers::ReceiverStream::new(rx);
        return Ok(tonic::Response::new(Box::pin(response) as ResponseStream));
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn check_dir_exists_test() {
        let commands = vec!["/FdD:\\WorkSpace\\turbobuild\\target\\debug\\test.pdb".to_string()];

        let handle = {
            let rt =  crate::common::RUNTIME.lock().unwrap();
            rt.handle().clone()
        };

        handle.spawn(async move {
            println!("run check_dir_exists test in runtime.");
            crate::communicate::unpackager::Receiver::check_dir_exists(&"test_dir".to_string(), &commands).await;
        });
    }

    #[test]
    fn check_dir_exists_test_args() {
        let commands = vec![
            "/c",
            "/I",
            "G:\\OpenSource\\llvm-project\\build\\lib\\Testing\\Annotations",
            "/I",
            "G:\\OpenSource\\llvm-project\\llvm\\lib\\Testing\\Annotations",
            "/I",
            "G:\\OpenSource\\llvm-project\\build\\include",
            "/I",
            "G:\\OpenSource\\llvm-project\\llvm\\include",
            "/Zi",
            "/nologo",
            "/W4",
            "/WX-",
            "/diagnostics:column",
            "/MP",
            "/Od",
            "/Ob0",
            "/Oi",
            "/D",
            "_UNICODE",
            "/D",
            "UNICODE",
            "/D",
            "WIN32",
            "/D",
            "_WINDOWS",
            "/D",
            "UNICODE",
            "/D",
            "_UNICODE",
            "/D",
            "CMAKE_INTDIR=\\\"Debug\\\"",
            "/Zc:preprocessor",
            "/Gm-",
            "/RTC1",
            "/MDd",
            "/GS",
            "/fp:precise",
            "/Zc:wchar_t",
            "/Zc:forScope",
            "/Zc:inline",
            "/GR-",
            "/std:c++17",
            "/FoLLVMTestingAnnotations.dir\\Debug\\",
            "/FdG:\\OpenSource\\llvm-project\\build\\Debug\\lib\\LLVMTestingAnnotations.pdb",
            "/external:W4",
            "/Gd",
            "/TP",
            "/errorReport:prompt",
            "/we4238",
            "/Zc:__cplusplus",
            "/bigobj",
            "-w14062",
            "/Gw",
            "/EHs-c-",
            "/TP",
            "G:\\OpenSource\\llvm-project\\build\\lib\\Testing\\Annotations\\LLVMTestingAnnotations.dir\\Debug\\Annotations.i"
        ].iter().map(|item| item.to_string()).collect::<Vec<String>>();

        let handle = {
            let rt =  crate::common::RUNTIME.lock().unwrap();
            rt.handle().clone()
        };


        handle.spawn(async move {
            println!("run check_dir_exists test in runtime.");
            crate::communicate::unpackager::Receiver::check_dir_exists(&"test_dir".to_string(), &commands).await;
        });

    }
}