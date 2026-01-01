
use std::io::Write;
use tokio::io::AsyncWriteExt;

use crate::communicate::syscallredirectpipe::MirrorSysCall;
use windows_sys::Win32 as win;

#[allow(non_camel_case_types)]
pub mod package {
    include!("../../proto/pack.rs");
}

pub struct CHANNEL {
    pub namedpipe_to_grpc_tx: std::sync::Arc<Option<tokio::sync::mpsc::Sender<MirrorSysCall>>>,
    pub namedpipe_to_grpc_rx: std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::Receiver<MirrorSysCall>>>>,
}

pub static NAMEDPIPE_TO_GRPC_CHANNEL: std::sync::LazyLock<CHANNEL> = std::sync::LazyLock::new(|| {

    let (tx, rx) = tokio::sync::mpsc::channel(128);
    let namedpipe_to_grpc_tx =  std::sync::Arc::new(Some(tx));
    let namedpipe_to_grpc_rx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(rx)));

    return CHANNEL {
        namedpipe_to_grpc_tx,
        namedpipe_to_grpc_rx
    };
});

#[derive(Default, Clone)] 
pub struct Receiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
    crate_files_exist: std::sync::Arc<tokio::sync::Mutex<std::vec::Vec<String>>>,
}

impl Receiver {
    
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {

        let receiver = Receiver {
            common,
            crate_files_exist: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
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
            crate_files_exist: self.crate_files_exist.clone(),
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

    async fn transmit_file_handle(&self, request: tonic::Request<tonic::Streaming<package::FileTrRequest>>, tx: tokio::sync::mpsc::Sender<Result<package::FileTrResponse, tonic::Status>>) {
        use tokio_stream::StreamExt;
        let mut stream = request.into_inner();
            
        while let Some(request) = stream.next().await {
            if let Ok(request) = request {

                let mut reply = package::FileTrResponse {
                    error_code: 0,
                    error_message: "sync file success.".to_string(),
                };

                let name = request.name;
                let path = request.path;
                let project = request.project;
                
                log::debug!("{} transmit file handle name: {}, path: {}", project, name, path);
        
                let file_type = request.file_type;
                let solution = request.solution;
                let content = request.content;
                
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
                else if file_type == package::FileType::Sourcefiles as i32 {
                    let ret = Self::storage(&solution, &path, &content).await;
                    if let Err(err) = ret {
                        log::error!("transmit file handle save source files failed: {}", err);
                        reply.error_code = 1;
                        reply.error_message = err;
                    };
                }
                else if file_type == package::FileType::Precompiledsrcfiles as i32 {
                    let ret = Self::storage(&solution, &path, &content).await;
                    if let Err(err) = ret {
                        log::error!("transmit file handle save precompiled src files failed: {}", err);
                        reply.error_code = 1;
                        reply.error_message = err;
                    };
                }
                else if file_type == package::FileType::Kits as i32 {
                        
                }
                else {
                    log::debug!("unknown file type: {}", file_type);
                }

                if tx.send(Ok(reply.clone())).await.is_err() {
                    log::error!("tx send error.");
                    break;
                };
            }
            else {
                log::error!("transmit file handle request error");
                break;
            }
        }
    }
    
    async fn transmit_task_handle(&self, request: package::CompileTrRequest, tx: tokio::sync::mpsc::Sender<Result<package::CompileTrResponse, tonic::Status>>) {

        let solution = request.solution;
        let solution_ = solution.clone();
        let project = request.project;
        let file = request.file;
        let compiler = request.compiler;
        let compiler_ = compiler.clone();
    
        let commands = request.commands;
        let content = request.content;
        
        log::trace!("transmit compile handle project: {}, file: {}, compiler: {}, commands size: {}, content size: {}KB.", &project, file, compiler, commands.len(), &content.len() / 1024 );

        if !content.is_empty() && !file.is_empty() {
            let ret = Self::storage(&solution_, &file, &content).await;

            let mut reply = package::CompileTrResponse {
                progress: package::CompileProgress::Filetransfer.into(),
                out: Vec::new(),
                err: Vec::new(),
                results: Vec::new(),
                status: 0,
                tips: "transmit do save file success.".to_string(),
            };

            if ret.is_err() {
                reply.status = 1;
                reply.tips = ret.unwrap_err();
            }

            tx.send(Ok(reply)).await.unwrap_or_else(|err| log::error!("tx send failed: {:?}", err));
        }

        if !compiler.is_empty() && !commands.is_empty() {
            
            let handle = Self::check_dir_exists(&solution, &request.working_dir, &commands).await;

            let compiler_input = crew::compiler::model::CompilerInput {
                solution: std::ffi::OsString::from(solution),
                project: std::ffi::OsString::from(project.clone()),
                compiler_path: std::ffi::OsString::from(compiler),
                compiler_working_dir: std::ffi::OsString::from(&request.working_dir),
                compiler_commands: commands.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                build_and_compiler_type: std::ffi::OsString::from(request.variety),
                envs: request.envs.iter().map(|env| (std::ffi::OsString::from(env.key.clone()), std::ffi::OsString::from(env.value.clone()))).collect(),
            };

            let tx_ = tx.clone();
            let output_callback = move |reply: package::CompileTrResponse| {
                let tx = tx_.clone();
                async move {
                    for result in &reply.results {
                        log::trace!("transmit compile handle return file: {}", result.file);
                    }

                    tx.send(Ok(reply)).await.unwrap_or_else(|err| log::error!("tx send failed: {:?}", err));
                }
            };
            handle.await.unwrap();
            Self::cocrew_execute(&compiler_input, output_callback).await;
            log::debug!("transmit compile task handle execute done, {} return file: {}", &project.clone(), file);
        }

        if compiler_.is_empty() || commands.is_empty() {
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

    async fn transmit_redirect_handle(&self, request: tonic::Request<tonic::Streaming<package::LocalRedirect>>, tx: tokio::sync::mpsc::Sender<Result<package::RemoteRedirect, tonic::Status>>) {
        
        use tokio_stream::StreamExt;
        let self_= self.clone();
        let self__ = self_.clone();
        let _read_handle = tokio::spawn(async move {
            log::debug!("transmit redirect handle read task start.");
            let mut stream = request.into_inner();
            let responder = crate::communicate::syscallredirectpipe::GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_tx.as_ref();

            while let Some(request) = stream.next().await {
                if let Ok(real) = request {
                    
                    log::debug!("transmit redirect real result: id {:?} api: {:?} params: {:?}", real.cid, real.api, real.params);

                    for intermediate in real.files {
                        //TODO: what time to remove file from crate_files_exist?
                        let exist = { self_.crate_files_exist.lock().await.contains(&intermediate.file) };
                        if !exist {
                            {self_.crate_files_exist.lock().await.push(intermediate.file.clone());}

                            let file = tokio::fs::OpenOptions::new()
                                .create(true)
                                .share_mode(win::Storage::FileSystem::FILE_SHARE_READ | win::Storage::FileSystem::FILE_SHARE_WRITE | win::Storage::FileSystem::FILE_SHARE_DELETE)
                                .write(true)
                                .open(format!("{}{}", &intermediate.file, ".tmp"))
                                .await;

                            match file {
                                Ok(mut file) => {
                                    file.write_all(&intermediate.content).await.unwrap();
                                    file.flush().await.unwrap();
                                    drop(file);

                                    match tokio::fs::rename(format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file).await {
                                        Ok(_) => {},
                                        Err(err) => {
                                            if err.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32) {
                                                std::fs::rename(format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file).unwrap_or_else(|err| {
                                                    panic!("rename file failed: from {} to {}, {}", format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file, err);
                                                });
                                            }
                                            else {
                                                log::error!("rename file failed: from {} to {}, {}", format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file, err);
                                            }
                                        }
                                    }
                                },
                                Err(err) => {
                                    if err.kind() == std::io::ErrorKind::NotFound {

                                        let filepath = std::path::PathBuf::from(&intermediate.file);
                                        if let Some(parent) = filepath.parent() {
                                            if !parent.exists() {
                                                tokio::fs::create_dir_all(parent).await.expect(&format!("create dir failed: {}", parent.display()));
                                            }
                                        }

                                        let mut file = tokio::fs::OpenOptions::new()
                                            .create(true)
                                            .share_mode(win::Storage::FileSystem::FILE_SHARE_READ | win::Storage::FileSystem::FILE_SHARE_WRITE | win::Storage::FileSystem::FILE_SHARE_DELETE)
                                            .write(true)
                                            .open(&intermediate.file)
                                            .await.expect(&format!("create file failed: {}", &intermediate.file));
                                        
                                        file.write_all(&intermediate.content).await.unwrap();
                                    }
                                    else {
                                        panic!("create file failed: {:?}, {}", &intermediate.file, err);
                                    }
                                }
                            }
                        }
                    }

                    let mut dir_exists = false;
                    let mut replace = String::new();
                    real.params.iter().for_each(|param| {
                        if param.key == "exists" && param.value ==  "true" {
                            dir_exists = true;
                        }
                        if param.key == "replace" {
                            replace = param.value.clone();
                        }
                    });

                    if dir_exists && !replace.is_empty() && !std::path::Path::new(&replace).exists() {
                        log::info!("transmit redirect handle create replace dir: {}", replace);
                        std::fs::create_dir_all(&replace).unwrap_or_else(|err| {
                            panic!("create replace dir failed: {} {}", replace, err);
                        });
                    }

                    let command_result = MirrorSysCall {
                        cid: real.cid,
                        api: real.api.clone(),
                        args: real.params.iter().map(|param| (param.key.clone(), param.value.clone())).collect(),
                    };
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    match responder.send(command_result) {
                        Ok(_) => {
                            //log::debug!("transmit redirect handle send callback: {:?} {}", real.api, real.cid);
                        },
                        Err(err) => {
                            log::error!("transmit redirect handle send callback failed: {:?}", err);
                        },
                    };
                }
                else if let Err(err) = request {
                    log::error!("transmit redirect handle inbound error: {:?}", err);
                    break;
                }
            }
         
            log::debug!("transmit redirect handle read task end.");
            
        });

        //receive syscall messages from mpsc and send syscall to crew by grpc.
        let _write_handle = tokio::spawn(async move {

            let responder = crate::communicate::syscallredirectpipe::GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_tx.as_ref();
            let channel = { NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_rx.lock().await.take() };

            if let Some(mut rx) = channel {
                while let Some(syscall) = rx.recv().await {
                    log::debug!("transmit redirect handle received message: {:?}, remaining: {}", &syscall, rx.len());

                    if syscall.api == "NtCreateFile" {

                        if let Some((_, expect)) = syscall.args.get_key_value("expect") {
                            //file exist in replica dir, so do not obtain file from crew again. direct return success.
                            let exist = { self__.crate_files_exist.lock().await.contains(&expect) };
                            if exist {
                                match responder.send(syscall) {
                                    Ok(_) => {},
                                    Err(err) => {
                                        log::error!("transmit redirect handle send callback failed: {:?}", err);
                                    },
                                }
                                log::error!("file already exists in replica dir, skipping obtain from crew");
                                continue;
                            }
                        }
                    }

                    let reply = package::RemoteRedirect {
                        cid: syscall.cid,
                        api: syscall.api.clone(), 
                        params: syscall.args.iter().map(|(k, v)| package::Params { key: k.clone(), value: v.clone() }).collect(),
                    };
                    
                    if tx.send(Ok(reply)).await.is_ok() {
                    }
                    else {
                        log::warn!("transmit redirect handle send syscall message to crew failed. id: {}", syscall.cid);
                    }
                }
                log::debug!("transmit redirect handle write task end.");
            }
        });
    }

    async fn storage(solution: &str, path: &str, content: &[u8]) -> Result<(), String> {
        let now = std::time::Instant::now();

        if solution.is_empty() || path.is_empty() {
            log::error!("transmit storage project name or path is empty.");
            return Err("project or path param is empty, so do nothing".to_string());
        }
        else {
            let project: crew::replica::project::Property = crew::replica::project::Property::new(solution, path);
            let path = project.fetch_local_replica_project_path();
    
            if path.extension() == Some(&std::ffi::OsStr::new("zip")) {
                Self::extract(&path.to_str().unwrap(), &content).await;
            }
            else {
                let file = match std::fs::File::create(&path) {
                    Ok(file) => Ok(file),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            let parent = path.parent().unwrap();
                            if !parent.exists() {
                                std::fs::create_dir_all(parent).unwrap();
                                let file = std::fs::File::create(&path);
                                file
                            }
                            else {
                                Err(err)
                            }
                        }
                        else {
                            log::error!("transmit storage file create failed: {:?}, {}", path, err);
                            Err(err)
                        }
                    }
                };
                
                //same source file may being used by another process. compiler open and current write at same time.
                if let Ok(mut file) = file {
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
        }

        log::info!("transmit storage file done. elapsed: {:?}", now.elapsed());
        return Ok(());   
    }

    async fn extract(path: &str, content: &[u8]) {
        if path.ends_with(".zip") {
            let dir = &path[..(path.len() - ".zip".len())];
            let cursor = std::io::Cursor::new(content);
            let mut zip = zip::ZipArchive::new(cursor).unwrap();
            
            match zip.extract(dir) {
                Ok(_) => {
                    log::trace!("extract zip file done: {} {:?}", dir, zip.file_names().collect::<Vec<&str>>());
                },
                Err(err) => {
                    log::error!("extract zip file failed. {} {} {:?}", dir, err, zip.file_names().collect::<Vec<&str>>());
                }
            }
        }
        else {
            let cursor = std::io::Cursor::new(content);
            let mut zip = zip::ZipArchive::new(cursor).unwrap();
            match zip.extract(path) {
                Ok(_) => {
                    log::trace!("extract zip file done: {} {:?}", path, zip.file_names().collect::<Vec<&str>>());
                },
                Err(err) => {
                    log::error!("extract zip file failed. {} {} {:?}", path, err, zip.file_names().collect::<Vec<&str>>());
                }
            }
        }
    }

    async fn cocrew_execute<Func, Fut>(input: &crew::compiler::model::CompilerInput, sender: Func)
        where Func: Fn(package::CompileTrResponse) -> Fut + Send + Sync + Clone + 'static,
              Fut: std::future::Future<Output = ()> + Send
    {
        let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<crew::compiler::model::CompiledResults>(128);
        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<crew::compiler::model::CompiledResults>(128);
    
        let out_err_stream = crate::compiler::msvc::CompiledResultsStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let sender_ = sender.clone();

        let handle = tokio::spawn(async move {

            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Compilestart.into(),
                out: Vec::new(),
                err: Vec::new(),
                results: Vec::new(),
                status: 0,
                tips: "transmit do compile start.".to_string(),
            };
            sender_(reply).await;

            while let Some(results) = out_receiver.recv().await {
                for result in results {
                    let mut intermediates = Vec::new();
                    let _source = result.source_file;
      
                    if let Some(obj) = result.obj {
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

                    let reply = package::CompileTrResponse {
                        progress: package::CompileProgress::Compiling.into(),
                        out: Vec::new(),
                        err: Vec::new(),
                        results: intermediates,
                        status: 0,
                        tips: "transmit do compile success.".to_string(),
                    };

                    sender_(reply).await;
                }
            }
            log::debug!("transmit compile handle out stream end.");
        });

        tokio::spawn(async move {
            while let Some(results) = err_receiver.recv().await {
                
            }
            log::debug!("transmit compile handle err stream end.");
        });

        let input_ = input.to_owned();
        let buildhandle = tokio::task::Builder::new()
        .name("cocrew_build")
        .spawn_blocking(|| {
            let (output, _) = crate::compiler::interface::cocrew_build(input_, out_err_stream);
            output
        }).unwrap();

        let output = buildhandle.await.unwrap();
        if output.status == 0 {
            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Compiledone.into(),
                out: output.out.to_vec(),
                err: output.err.to_vec(),
                results: Vec::new(),
                status: output.status,
                tips: "transmit do compile success.".to_string(),
            };
            sender(reply).await;
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
            sender(reply).await;
        }
        handle.await.unwrap();

    }

    //create dir for .pdb and .obj, if parent dir not exist, .pdb and .obj file can not be generated.
    pub async fn check_dir_exists(solution: &String, working_dir: &String, commands: &Vec<String>) -> tokio::task::JoinHandle<()> {

        let solution = solution.clone();
        let commands = commands.clone();
        let working_dir = working_dir.clone();

        let handel = tokio::spawn(async move {
            let createdir = |dir: &std::path::Path| {
                if !dir.exists() {
                    if let Err(err)  = std::fs::create_dir_all(&dir) {
                        log::error!("create all dir failed: {:?} {}", dir, err);
                    }
                }
            };

            let replica = std::path::PathBuf::from(tools::utils::access_replica_dir());
            let split = |pdb_or_obj: &std::path::Path, solution: &String| {
                if solution.is_empty() {
                    return Some(pdb_or_obj.to_path_buf());
                }
                else {
                    let components = pdb_or_obj.components().collect::<Vec<_>>();
                    if let Some(index) = components.iter().position(|item| item.as_os_str().to_str().unwrap() == solution) {
                        let result: std::path::PathBuf = components[index + 1..].iter().collect();
                        let path = replica.join("Project").join(solution).join(result);
                        return Some(path);
                    }
                    else {
                        log::error!("not find solution in path: {:?} solution: {}.", pdb_or_obj, solution);
                        return None;
                    }            
                }
            };

            let path = commands.iter().find(|&item| item.starts_with("/Fd")).map(|item| item.clone());

            match path {
                Some(mut path) => {
                    let pdb = path.split_off(3);
                    let pdb = std::path::Path::new(&pdb);

                    if pdb.is_absolute() {
                        if let Some(path) = split(pdb, &solution) {
                            if path.extension().is_none() {
                                createdir(path.as_path());
                            }
                            else {
                                let parent = path.parent().unwrap();
                                createdir(parent);
                            }
                        }
                        else {
                            log::error!("split path failed, so not create pdb dir.");
                        }
                    }
                    else {
                        if let Some(path) = split(std::path::Path::new(&working_dir), &solution) {
                            let path = path.join(pdb);
                            if path.extension().is_none() {
                                createdir(path.as_path());
                            }
                            else {
                                let path = path.parent().unwrap();
                                createdir(path);
                            }
                        }
                        else {
                            log::error!("split path failed, so not create pdb dir.");
                        }
                    }
                },
                None => {},
            }

            let path = commands.iter().find(|&item| item.starts_with("/Fo")).map(|item| item.clone());

            match path {
                Some(mut path) => {
                    let obj = path.split_off(3);
                    let obj = std::path::Path::new(&obj);

                    if obj.is_absolute() {
                        if let Some(path) = split(obj, &solution) {
                            if path.extension().is_none() {
                                createdir(path.as_path());
                            }
                            else {
                                let path = path.parent().unwrap();
                                createdir(path);
                            }
                        }
                        else {
                            log::error!("split path failed, so not create obj dir.");
                        }
                    }
                    else {
                        if let Some(path) = split(std::path::Path::new(&working_dir), &solution) {
                            let path = path.join(obj);
                            if path.extension().is_none() {
                                createdir(&path);
                            }
                            else {
                                let path = path.parent().unwrap().to_owned();
                                createdir(&path);
                            }
                        }
                        else {
                            log::error!("split path failed, so not create obj dir.");
                        }
                    }
                },
                None => {},
            }
        });
        return handel;
    }
    
}

type ResponseTaskStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::CompileTrResponse, tonic::Status>> + Send>>;
type ResponseFileStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::FileTrResponse, tonic::Status>> + Send>>;

type RemoteRedirectStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::RemoteRedirect, tonic::Status>> + Send>>;

#[tonic::async_trait]
impl package::communicate_server::Communicate for Receiver {

    type transmit_fileStream = ResponseFileStream;
    async fn transmit_file(&self, request: tonic::Request<tonic::Streaming<package::FileTrRequest>>) -> core::result::Result<tonic::Response<Self::transmit_fileStream>, tonic::Status> {
        log::debug!("sync request transmit file. from: {:?}", request.remote_addr());
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        let self_ = self.clone();
        let _ = tokio::task::spawn(async move {
            self_.transmit_file_handle(request, tx).await;
        });

        let response = tokio_stream::wrappers::ReceiverStream::new(rx);
        return Ok(tonic::Response::new(Box::pin(response) as ResponseFileStream));
    }
    
    type transmit_taskStream = ResponseTaskStream;
    async fn transmit_task(&self, request: tonic::Request<package::CompileTrRequest>) -> core::result::Result<tonic::Response<Self::transmit_taskStream>, tonic::Status> {
        
        let rt_compile = request.into_inner();
        log::debug!("sync request transmit compiler: {:?}", rt_compile.compiler);
        log::debug!("commands: {:?}", rt_compile.commands);
        
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let self_ = self.clone();
        let _ = tokio::spawn(async move {
            self_.transmit_task_handle(rt_compile, tx).await;
        });

        let response = tokio_stream::wrappers::ReceiverStream::new(rx);
        return Ok(tonic::Response::new(Box::pin(response) as ResponseTaskStream));
    }

    type transmit_redirectStream = RemoteRedirectStream;
    async fn transmit_redirect(&self, request: tonic::Request<tonic::Streaming<package::LocalRedirect>>) -> core::result::Result<tonic::Response<Self::transmit_redirectStream>, tonic::Status> {
        log::debug!("sync request transmit redirect: {:?}", request.remote_addr());

        let exist = { NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_rx.lock().await.is_some() };

        if  exist {
            let (tx, rx) = tokio::sync::mpsc::channel(256);
        
            let self_ = self.clone();
            self_.transmit_redirect_handle(request, tx).await;

            let response = tokio_stream::wrappers::ReceiverStream::new(rx);
            return Ok(tonic::Response::new(Box::pin(response) as RemoteRedirectStream));
        }
        else  {
            return Err(tonic::Status::internal("namedpipe_rx receiver is not available."));
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn check_dir_exists_test() {
        let commands = vec!["/FdD:\\WorkSpace\\turbobuild\\target\\debug\\test.pdb".to_string()];

        let handle = {
            let rt =  crate::common::COCREW_RUNTIME.lock().unwrap();
            rt.handle().clone()
        };

        handle.spawn(async move {
            println!("run check_dir_exists test in runtime.");
            crate::communicate::unpackager::Receiver::check_dir_exists(&"test_dir".to_string(), &"".to_string(), &commands).await;
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
            let rt =  crate::common::COCREW_RUNTIME.lock().unwrap();
            rt.handle().clone()
        };


        handle.spawn(async move {
            println!("run check_dir_exists test in runtime.");
            crate::communicate::unpackager::Receiver::check_dir_exists(&"llvm-project".to_string(), &"".to_string(), &commands).await;
        });

    }
}