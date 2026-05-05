
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

    let (tx, rx) = tokio::sync::mpsc::channel(1024);
    let namedpipe_to_grpc_tx =  std::sync::Arc::new(Some(tx));
    let namedpipe_to_grpc_rx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(rx)));

    return CHANNEL {
        namedpipe_to_grpc_tx,
        namedpipe_to_grpc_rx
    };
});

pub struct CompileResultSync {
    pub task_to_file_tx: tokio::sync::mpsc::Sender<crew::compiler::model::CompiledResult>,
    pub task_to_file_rx: tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crew::compiler::model::CompiledResult>>,
}

pub static TASK_TO_FILE_CHANNEL: std::sync::LazyLock<CompileResultSync> = std::sync::LazyLock::new(|| {

    let (task_to_file_tx, task_to_file_rx) = tokio::sync::mpsc::channel(1024);

    return CompileResultSync {
        task_to_file_tx: task_to_file_tx,
        task_to_file_rx: tokio::sync::Mutex::new(task_to_file_rx),
    };
});

#[derive(Default, Clone)] 
pub struct Receiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
    create_files_exist: std::sync::Arc<tokio::sync::Mutex<std::vec::Vec<String>>>,
    file_write_locks: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<tokio::sync::Mutex<()>>>>>,
    redirect_create_file_exists: std::sync::Arc<std::sync::RwLock<std::collections::HashSet<String>>>,
    redirect_query_directory_file_infos: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, QueryDirectoryFileInfos>>>,
}

#[derive(Debug, Clone)]
pub struct QueryDirectoryFileInfos {
    pub cids: Vec<u32>,
    pub fileinformation: String,
}

impl Receiver {
    
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {

        let receiver = Receiver {
            common,
            create_files_exist: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            file_write_locks: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            redirect_create_file_exists: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashSet::new())),
            redirect_query_directory_file_infos: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::<String, QueryDirectoryFileInfos>::new())),
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
            create_files_exist: self.create_files_exist.clone(),
            file_write_locks: self.file_write_locks.clone(),
            redirect_create_file_exists: self.redirect_create_file_exists.clone(),
            redirect_query_directory_file_infos: self.redirect_query_directory_file_infos.clone(),
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

        let tx_ = tx.clone();
        tokio::spawn(async move {
            while let Some(result) = TASK_TO_FILE_CHANNEL.task_to_file_rx.lock().await.recv().await {

                if let Some((file, context)) = result.obj {
                    let reply = package::FileTrResponse {
                        path: file.to_string_lossy().to_string(),
                        content: context,
                        error_code: 0,
                        error_message: result.source_file.to_string_lossy().to_string(),
                    };
                    tx_.send(Ok(reply)).await.unwrap();
                }

                if let Some((file, context)) = result.pdb {
                    let reply = package::FileTrResponse {
                        path: file.to_string_lossy().to_string(),
                        content: context,
                        error_code: 0,
                        error_message: result.source_file.to_string_lossy().to_string(),
                    };
                    tx_.send(Ok(reply)).await.unwrap();
                }

                if let Some((file, context)) = result.idb {
                    let reply = package::FileTrResponse {
                        path: file.to_string_lossy().to_string(),
                        content: context,
                        error_code: 0,
                        error_message: result.source_file.to_string_lossy().to_string(),
                    };
                    tx_.send(Ok(reply)).await.unwrap();
                }
            }
        });
            
        while let Some(request) = stream.next().await {
            if let Ok(request) = request {

                let mut reply = package::FileTrResponse {
                    path: "".to_string(),
                    content: Vec::new(),
                    error_code: 0,
                    error_message: "sync file success.".to_string(),
                };

                let name = request.name;
                let path = request.path;
                let project = request.project;
        
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
                else if file_type == package::FileType::Synctaskcount as i32 {
                    let count_str = String::from_utf8_lossy(&content);
                    if let Ok(count) = count_str.parse::<usize>() {
                        let mut compile_task = crate::compiler::msvc::COMPILE_TASK_COUNT.lock().unwrap();
                        let task = compile_task.get_mut(&project);
                        if let Some(task) = task {
                            if task.done >= count {
                                let tx_ = tx.clone();
                                let pdb = task.pdb.clone();
                                tokio::spawn(async move {
                                    Self::return_local_compile_result_pdbfiles(pdb,
                                        &std::ffi::OsString::from(solution),
                                        &std::ffi::OsString::from(path),
                                        tx_
                                    ).await;
                                });
                                compile_task.remove(&project);
                            }
                            else {
                                task.expected = count;
                            }
                        }
                        else {
                            compile_task.insert(project.clone(), crate::compiler::msvc::CompileTaskCount {
                                pdb: crate::compiler::msvc::ProgramDataBase::NonePDBPath,
                                expected: count,
                                done: 0,
                            });
                        }
                    }
                    else {
                        log::error!("parse sync task count failed: {}", count_str);
                    }
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
            
            let project_ = project.clone();
            let output_callback = move |reply: package::CompileTrResponse| {
                let tx = tx_.clone();
                let project_ = project_.clone();
                async move {
                    for result in &reply.results {
                        log::trace!("transmit compile handle {} return file: {}", project_, result.file);
                    }

                    tx.send(Ok(reply)).await.unwrap_or_else(|err| log::error!("tx send failed: {:?}", err));
                }
            };
            handle.await.unwrap();
            Self::cocrew_execute(&compiler_input, output_callback).await;
            //TODO: what is file used for?
            log::debug!("transmit compile task handle execute done, {} return file: {}", &project, file);
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

    async fn transmit_syscall_handle(&self, request: tonic::Request<tonic::Streaming<package::LocalSyscall>>, tx: tokio::sync::mpsc::Sender<Result<package::RemoteSyscall, tonic::Status>>) {
        
        use tokio_stream::StreamExt;
        let self_= self.clone();
        let self__ = self_.clone();
        let _read_handle = tokio::spawn(async move {
            log::debug!("transmit redirect handle read task start.");
            let mut stream = request.into_inner();
            let responder = crate::communicate::syscallredirectpipe::GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_tx.as_ref();

            while let Some(request) = stream.next().await {
                if let Ok(real) = request {

                    let create_files_exist = std::sync::Arc::clone(&self_.create_files_exist);
                    let file_write_locks = std::sync::Arc::clone(&self_.file_write_locks);
                    let redirect_create_file_exists = std::sync::Arc::clone(&self_.redirect_create_file_exists);
                    let redirect_query_directory_file_infos = std::sync::Arc::clone(&self_.redirect_query_directory_file_infos);

                    tokio::spawn(async move {
                        if real.api == "NtCreateFile" {
                            for intermediate in real.files {
                                //TODO: what time to remove file from crate_files_exist?
                                let exist = { create_files_exist.lock().await.contains(&intermediate.file) };
                                if !exist {
    
                                    let mut file_write_locks = file_write_locks.lock().await;
                                    let file_write_lock = file_write_locks.entry(intermediate.file.clone())
                                        .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(()))).clone();
    
                                    let file = tokio::fs::OpenOptions::new()
                                        .create_new(true)
                                        .share_mode(win::Storage::FileSystem::FILE_SHARE_READ | win::Storage::FileSystem::FILE_SHARE_WRITE | win::Storage::FileSystem::FILE_SHARE_DELETE)
                                        .write(true)
                                        .open(format!("{}{}", &intermediate.file, ".tmp"))
                                        .await;
        
                                    match file {
                                        Ok(mut file) => {
                                            let guard = file_write_lock.lock().await;
    
                                            file.write_all(&intermediate.content).await.unwrap();
                                            file.flush().await.unwrap();
    
                                            drop(file);
                                            match tokio::fs::rename(format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file).await {
                                                Ok(_) => {
                                                    {create_files_exist.lock().await.push(intermediate.file.clone());}
                                                    drop(guard);
                                                    file_write_locks.remove(&intermediate.file);
                                                },
                                                Err(err) => {
                                                    if err.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32) {
                                                        std::fs::rename(format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file).unwrap_or_else(|err| {
                                                            panic!("rename file failed: from {} to {}, {}", format!("{}{}", &intermediate.file, ".tmp"), &intermediate.file, err);
                                                        });
                                                    }
                                                    else {
                                                        //TODO: if rename failed, how to deal ?
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
                                                file.flush().await.unwrap();
                                            }
                                            else if err.kind() == std::io::ErrorKind::AlreadyExists {
                                                let guard = file_write_lock.lock().await;
                                                drop(guard);
                                                file_write_locks.remove(&intermediate.file);
                                            }
                                            else {
                                                panic!("create file failed: {:?}, {}", &intermediate.file, err);
                                            }
                                        }
                                    }
                                }
                                else {
                                    log::trace!("redirect handle file already exists in replica dir, skipping sync file: {} {}", real.cid, &intermediate.file);
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
        
                            if dir_exists && !replace.is_empty() {

                                redirect_create_file_exists.write().unwrap().insert(replace.clone());

                                if !std::path::Path::new(&replace).exists() {
                                    match std::fs::create_dir_all(&replace) {
                                        Ok(_) => {},
                                        Err(err) => {
                                            if err.kind() == std::io::ErrorKind::AlreadyExists {
                                                log::error!("create replace dir failed: {} {}", replace, err)
                                            }
                                            else {
                                                panic!("create replace dir failed: {} {}", replace, err);
                                            }
                                        },
                                    };
                                }
                            }
                        }
                        else if real.api == "NtQueryDirectoryFile" {

                            let mut path = String::new();
                            let mut fileinformation = String::new();
                            real.params.iter().for_each(|param| {
                                if param.key == "filehandle" {
                                    path = param.value.clone();
                                }
                                if param.key == "fileinformation" {
                                    fileinformation = param.value.clone();
                                }
                            });

                            let mut caches = redirect_query_directory_file_infos.write().unwrap();
                            if let Some(c) = caches.get_mut(&path) {

                                let cids = c.cids.clone();

                                c.cids.clear();
                                c.fileinformation = fileinformation.clone();

                                drop(caches);
                                for cid in cids {
                                    if cid != real.cid {
                                        let command_result = MirrorSysCall {
                                           cid: cid,
                                           api: real.api.clone(),
                                           args: std::collections::HashMap::from([
                                               ("fileinformation".to_string(), fileinformation.clone()),
                                           ]),
                                        };

                                       match responder.send(command_result) {
                                           Ok(_) => {
                                               log::debug!("transmit redirect handle send query directory cache callback: {} {}", real.api, cid);
                                           },
                                           Err(err) => {
                                               log::error!("transmit redirect handle send query directory cache callback failed: {:?}", err);
                                           },
                                       };
                                    }
                                }
                            }
                        }

                        let command_result = MirrorSysCall {
                            cid: real.cid,
                            api: real.api.clone(),
                            args: real.params.iter().map(|param| (param.key.clone(), param.value.clone())).collect(),
                        };

                        match responder.send(command_result) {
                            Ok(_) => {
                                //log::debug!("transmit redirect handle send callback: {:?} {}", real.api, real.cid);
                            },
                            Err(err) => {
                                log::error!("transmit redirect handle send callback failed: {:?}", err);
                            },
                        };
                    });
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

                    if syscall.api == "NtCreateFile" {
                        if let Some((_, expect)) = syscall.args.get_key_value("expect") {
                            //file exist in replica dir, so do not obtain file from crew again. direct return success.
                            let exist = { self__.create_files_exist.lock().await.contains(&expect) };
                            if exist {
                                log::error!("file already exists in replica dir, skipping obtain from crew, {}", expect);
                                match responder.send(syscall) {
                                    Ok(_) => {},
                                    Err(err) => {
                                        log::error!("transmit redirect handle send callback failed: {:?}", err);
                                    },
                                }
                                continue;
                            }
                        }
                        else if let Some((_, _)) = syscall.args.get_key_value("exists") {
                            if let Some((_, path)) = syscall.args.get_key_value("objectname") {
                                let exist = { self__.redirect_create_file_exists.read().unwrap().contains(path) };
                                if exist {

                                    let reply = MirrorSysCall {
                                        cid: syscall.cid,
                                        api: syscall.api,
                                        args: syscall.args.iter().map(|item| 
                                            if item.0 == "exists" {
                                                (item.0.clone(), "true".to_string())
                                            }
                                            else {
                                                (item.0.clone(), item.1.clone())
                                            }   
                                        ).collect(),
                                    };

                                    match responder.send(reply) {
                                        Ok(_) => {},
                                        Err(err) => {
                                            log::error!("transmit redirect handle send callback failed: {:?}", err);
                                        },
                                    }
                                    continue;   
                                }
                            }
                        }
                    }
                    else if syscall.api == "NtQueryDirectoryFile" {
                        if let Some((_, path)) = syscall.args.get_key_value("filehandle") {
                            let mut caches = self__.redirect_query_directory_file_infos.write().unwrap();
                            if let Some(c) = caches.get_mut(path) {
                                if c.fileinformation.is_empty() {
                                    c.cids.push(syscall.cid);
                                    drop(caches);
                                    continue;
                                }
                                else {
                                    let reply = MirrorSysCall {
                                        cid: syscall.cid,
                                        api: syscall.api.clone(),
                                        args: std::collections::HashMap::from([
                                            ("fileinformation".to_string(), c.fileinformation.clone()),
                                        ]),
                                    };

                                    drop(caches);

                                    match responder.send(reply) {
                                        Ok(_) => {
                                            log::debug!("transmit redirect handle send query directory cache callback: {} {}", syscall.api, syscall.cid);
                                        },
                                        Err(err) => {
                                            log::error!("transmit redirect handle send query directory cache callback failed: {:?}", err);
                                        },
                                    };
                                    continue;
                                }
                            }
                            else {
                                caches.insert(path.clone(), QueryDirectoryFileInfos {
                                    cids: vec![syscall.cid],
                                    fileinformation: String::new(),
                                });
                                drop(caches);
                            }
                        }
                    }

                    let reply = package::RemoteSyscall {
                        cid: syscall.cid,
                        api: syscall.api.clone(), 
                        params: syscall.args.iter().map(|(k, v)| package::Params { key: k.clone(), value: v.clone() }).collect(),
                    };
                    
                    match tx.send(Ok(reply.clone())).await {
                        Ok(_) => {
                        },
                        Err(err) => {
                            log::warn!("transmit redirect handle send syscall message to crew failed. id: {} , {:?}", syscall.cid, err);
                        }
                    };
                }
                log::debug!("transmit redirect handle write task end.");
            }
            else {
                panic!("transmit redirect handle obtain namedpipe to grpc channel failed.");
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
                                log::debug!("transmit storage create parent dir: {:?}", parent);
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
                        },
                        Err(err) => {
                            log::error!("transmit storage file failed. {:?} {:?}", path, err)
                        }
                    }
                }
            }
        }
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
        let project_ = input.project.to_string_lossy().into_owned();
        let project__ = project_.clone();
        let outputhandle = tokio::spawn(async move {

            let reply = package::CompileTrResponse {
                progress: package::CompileProgress::Compilestart.into(),
                out: Vec::new(),
                err: Vec::new(),
                results: Vec::new(),
                status: 0,
                tips: "transmit do compile start.".to_string(),
            };
            sender_(reply).await;

            let mut joins = tokio::task::JoinSet::new();

            while let Some(results) = out_receiver.recv().await {
                for result in results {
                    let mut intermediates = Vec::new();

                    let source = result.source_file;
                    
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
                        out: source.to_str().unwrap_or("").as_bytes().to_vec(),
                        err: Vec::new(),
                        results: intermediates,
                        status: 0,
                        tips: "transmit do compile success.".to_string(),
                    };

                    let sender_ = sender_.clone();
                    joins.spawn(async move {
                        sender_(reply).await;
                    });
                }
            }

            joins.join_all().await;
            log::debug!("transmit compile handle {} out stream end.", project_);
        });

        tokio::spawn(async move {
            while let Some(results) = err_receiver.recv().await {
                
            }
            log::debug!("transmit compile handle {} err stream end.", project__);
        });

        let input_ = input.to_owned();
        let buildhandle = tokio::task::Builder::new()
        .name("cocrew_build")
        .spawn_blocking(|| {
            let (output, _) = crate::compiler::interface::cocrew_build(input_, out_err_stream);
            output
        }).unwrap();

        let output = buildhandle.await.unwrap();
        //wait compile output send back done.
        //outputhandle.await.unwrap();
        
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
                status: output.status,
                tips: "transmit do compile failed.".to_string(),
            };
            sender(reply).await;
        }
    }

    //create dir for .pdb and .obj, if parent dir not exist, .pdb and .obj file can not be generated.
    pub async fn check_dir_exists(solution: &String, working_dir: &String, commands: &Vec<String>) -> tokio::task::JoinHandle<()> {

        let solution = solution.clone();
        let commands = commands.clone();
        let working_dir = working_dir.clone();

        let handel = tokio::spawn(async move {
            let createdir = |dir: &std::path::Path| {
                if !dir.exists() {
                    log::debug!("check dir exists create dir: {:?}", dir);
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
    
    async fn return_local_compile_result_pdbfiles(program_database: crate::compiler::msvc::ProgramDataBase, solution_name: &std::ffi::OsString, 
        origin_working_dir: &std::ffi::OsString, tx: tokio::sync::mpsc::Sender<Result<package::FileTrResponse, tonic::Status>>) {
        use tokio::io::AsyncReadExt;

        let mut result = std::path::PathBuf::from("");
        match &program_database {
            crate::compiler::msvc::ProgramDataBase::PathWithPDBName(path) => {
                result = path.clone();
            },
            crate::compiler::msvc::ProgramDataBase::PathWithoutPDBName(dir) => {
                result = dir.join("vc143");
                result.set_extension("pdb");
            },
            _ => {
                log::warn!("fetch result pdb file path failed.");
            }
        };
    
        log::trace!("compile result program database path: {:?}", result);

        if result.exists() {
            let solution_name_ = solution_name.clone();
            let origin_working_dir_ = origin_working_dir.clone();
            let tx_ = tx.clone();
            let mut result_ = result.clone();
            tokio::spawn(async move {
                match tokio::fs::File::open(&result).await {
                    Ok(file) => {
                        let mut contents = Vec::new();
                        let mut file = tokio::io::BufReader::new(file);
                        let _ = file.read_to_end(&mut contents).await.unwrap();
                        let origin = crate::compiler::msvc::repair_original_path(&solution_name_, &origin_working_dir_, &result);

                        let reply = package::FileTrResponse {
                            path: origin.to_string_lossy().to_string(),
                            content: contents,
                            error_code: 0,
                            error_message: "sync file success.".to_string(),
                        };

                        tx_.send(Ok(reply)).await.unwrap();
                    },
                    Err(error) => {
                        if error.kind() == std::io::ErrorKind::NotFound {
                            log::warn!(".pdb file path is not found. path: {:?}", result);
                        }
                        else {
                            log::warn!(".pdb file read failed. {:?}, {:?}.", error, result);
                        }
                    }
                }
            });
            
            let solution_name_ = solution_name.clone();
            let origin_working_dir_ = origin_working_dir.clone();
            let tx_ = tx.clone();
            tokio::spawn(async move {
                result_.set_extension("idb");
                match tokio::fs::File::open(&result_).await {
                    Ok(file) => {
                        let mut contents = Vec::new();
                        let mut file = tokio::io::BufReader::new(file);
                        let _ = file.read_to_end(&mut contents).await.unwrap();
                        let origin = crate::compiler::msvc::repair_original_path(&solution_name_, &origin_working_dir_, &result_);     
    
                        let reply = package::FileTrResponse {
                            path: origin.to_string_lossy().to_string(),
                            content: contents,
                            error_code: 0,
                            error_message: "sync file success.".to_string(),
                        };
    
                        tx_.send(Ok(reply)).await.unwrap();
                    },
                    Err(error) => {
                        if error.kind() == std::io::ErrorKind::NotFound {
                            //log::warn!(".idb file path is not found.");
                        }
                        else {
                            log::warn!(".idb file read failed. {:?}", error);
                        }
                    },
                }
            });
        }
        else {
            log::warn!("pdb file is not found, path: {:?}.", result);
        }
    }
}

type ResponseTaskStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::CompileTrResponse, tonic::Status>> + Send>>;
type ResponseFileStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::FileTrResponse, tonic::Status>> + Send>>;

type RemoteRedirectStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<package::RemoteSyscall, tonic::Status>> + Send>>;

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

    type transmit_syscallStream = RemoteRedirectStream;
    async fn transmit_syscall(&self, request: tonic::Request<tonic::Streaming<package::LocalSyscall>>) -> core::result::Result<tonic::Response<Self::transmit_syscallStream>, tonic::Status> {
        log::debug!("sync request transmit redirect: {:?}", request.remote_addr());

        let exist = { NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_rx.lock().await.is_some() };

        if  exist {
            let (tx, rx) = tokio::sync::mpsc::channel(256);
        
            let self_ = self.clone();
            self_.transmit_syscall_handle(request, tx).await;

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