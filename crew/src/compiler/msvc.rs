
//#[cfg(target_os = "windows")]
//extern crate regex;

#[derive(Clone)]
pub struct MSVC {
    pub work_env: crate::platform::windows::WindowsCompilerEnv,
    pub runtime: std::sync::Arc<tokio::runtime::Handle>,
    pub sender: std::sync::Arc<std::sync::Mutex<crate::communicate::distributor::Distributor>>,
}

pub struct StdOut {
    pub out: Box<tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>>,
    pub err: Box<tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStderr>>>,
    pub child: tokio::process::Child,
}

pub struct FileOut {
    pub out: Box<std::io::Lines<std::io::BufReader<std::process::ChildStdout>>>,
    pub err: Box<std::io::Lines<std::io::BufReader<std::process::ChildStderr>>>,
    pub child: std::process::Child,
}

pub enum OutType {
    Std(StdOut),
    File(FileOut),
}

//TODO common param in func should be move in msvc struct
use std::{io::Read, ops::{Add, Index}, sync::Arc};
use crate::compiler::model::{CompilerInput, CompilerOutput, CompiledResults, PrecompiledSource};
use std::io::BufRead;

impl crate::compiler::interface::Compiler for MSVC {
    fn request_compile(&self, compiler_input: CompilerInput) -> CompilerOutput {

        let self_ = self.clone();

        let handle = self.runtime.spawn(async move {
            let output = self_.request_msvc_compile(compiler_input).await;
            return output;
        });
        
        let output = tokio::task::block_in_place(||{
            let output = self.runtime.block_on(handle);
            return output;
        });
        
        match output {
            Ok(output) => {
                return output;
            },
            Err(err) => {
                log::error!("request compile failed. {:?}", err);
                return CompilerOutput::default();
            }
        }
    }
}

impl MSVC {

    async fn request_msvc_compile(self, compiler_input: CompilerInput) -> CompilerOutput {

        let compiler_commands = &compiler_input.compiler_commands;

        let working_path = std::path::PathBuf::from(&compiler_input.compiler_working_dir);

        if !working_path.exists() {
            let result = std::fs::create_dir_all(working_path.clone());
            match result {
                Ok(_) => {
                    log::info!("create dir in: {:?}", working_path);
                },
                Err(error) => {
                    log::warn!("can not create dir in: {:?}, error code: {:?}", working_path, error)
                },
            }
        }

        let exists_source_file_in_command = determine_whether_need_compile(compiler_commands.clone());
        
        let mut compiler_output = CompilerOutput::default();

        if exists_source_file_in_command {

            //TODO add expression to determine use local build or dist build
            
            if false {
                let (output, _) = request_local_compile(&compiler_input.compiler_path, &compiler_input.compiler_working_dir,
                                                            compiler_commands, compiler_input.build_and_compiler_type,
                                                            false);
                compiler_output.set(output);
            }
            else {
                if true {
                    // dist with preprocessed source
                    let now = std::time::Instant::now();
                    let output = self.request_multi_dist_once_compile(&compiler_input).await;
                    log::trace!("request_multi_dist_sync_once_compile elaspsed time: {:?}", now.elapsed());
                    //let output = request_dist_compile(&working_parameters.network_client, &compiler_path, &msvc_compile_input.compiler_working_dir, &compiler_commands.clone());
                    compiler_output.set(output);
                }
                else {
                    //dist with source file and include file
                    let output = request_dist_compile_with_source_and_include(&self.work_env, &compiler_input);
                    compiler_output.set(output);
                }
            }
            
            //TODO cache the result to redis or cloud

            return compiler_output;
        }
        else {
            return compiler_output;
        }
    }

    fn _enrich_compiler_commands(&self, compiler_input: &CompilerInput) -> Vec<std::ffi::OsString> {

        let mut commands = compiler_input.compiler_commands.clone();
    
        if compiler_input.build_and_compiler_type.to_string_lossy().contains("MSBuild") {
            let env =  &self.work_env;
    
            for include in &env.winkits_includes_path {
                if commands.iter().find(|&item| item.to_string_lossy().contains(&include.to_string_lossy().to_string())).is_none() {
                    commands.push(std::ffi::OsString::from("/I"));
                    commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
                }
            }

            if commands.iter().find(|&item| item.to_string_lossy().contains(&env.msvc_includes_path.to_string_lossy().to_string())).is_none() {
                commands.push(std::ffi::OsString::from("/I"));
                commands.push(std::ffi::OsString::from(format!("{}", self.work_env.msvc_includes_path.to_string_lossy())));
            }
    
            return commands;
        }
        else if  compiler_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
            return commands;
        }
        else {
            return commands;
        }
    }

    async fn request_multi_dist_once_compile(&self, compiler_input: &CompilerInput) -> CompilerOutput {

        let mut output = CompilerOutput::default();
        let now = std::time::Instant::now();

        let working_dir = &compiler_input.compiler_working_dir;
        let compiler_commands = &compiler_input.compiler_commands;

        let outtype = request_local_precompile(&compiler_input.compiler_path, working_dir, &self.merge_sdk_includes_into_commands(&compiler_commands), false);
        match outtype {
            Ok(out) => {
                match out {
                    crate::compiler::msvc::OutType::Std(_stdout) => {
                        //handle_compile_by_std(stdout);
                    },
                    crate::compiler::msvc::OutType::File(mut fileout) => {

                        let addr = self.sender.lock().unwrap().schedule();
                        output = self.handle_compile_by_filestream(&mut fileout, &compiler_input, &addr, &compiler_commands).await;

                        match fileout.child.wait() {
                            Ok(exit) => {
                                let code = exit.code().unwrap_or(-1);
                                log::info!("precompile status code: {:?}, elapsed time: {:?}", code, now.elapsed());
                            },
                            Err(err) => {
                                log::info!("precompile status code: {:?}, elapsed time: {:?}", err, now.elapsed());
                            }
                        }
                    },
                }
            },
            Err(err) => {
                log::warn!("precompile failed: {:?}", err);
                output.status = 1;
            },
        }

        log::debug!("local precompile and dist file and commmand done. elaspsed time: {:?}", now.elapsed());
        return output;
    }

    //Discarded function
    async fn request_dist_compile_from_file(&self, precompiled_result: &PrecompiledResult, addr: &str, source_files: Vec<String>, compiler_input: &CompilerInput)
        -> CompilerOutput {
        let now = std::time::Instant::now();

        let precompiled_files = self.load_and_transmit_precompiled_result(&source_files, &addr, &compiler_input.project, &precompiled_result).await;
        let len = precompiled_files.len();
        log::debug!("request dist sync precompiled source file. count: {:?}, addr: {}, elapsed time {:?}", len, addr, now.elapsed());
        let mut commands = compiler_input.compiler_commands.clone();
        for file in precompiled_files {
            commands.push(file);
        }
        
        let mut input = CompilerInput::from(compiler_input.clone());
        input.compiler_commands =  commands.to_owned();
        input.build_and_compiler_type = std::ffi::OsString::from("MSBuild_Precompile");

        let precompiled_suorce = crate::compiler::model::PrecompiledSource {
            contents: None,
            path: std::ffi::OsString::new(),
        };

        let output = self.request_dist_compile_from_stdout(&addr, &input, &precompiled_suorce).await;
        log::debug!("request dist compile with precompiled source files. count: {:?}, addr: {:?}, elapsed time {:?}", len, addr, now.elapsed());
        return output;
    }
    
    async fn request_dist_compile_with_command(&self, addr: &str, compiler_input: CompilerInput)
        -> CompilerOutput {

        let now = std::time::Instant::now();

        let precompiled_source = crate::compiler::model::PrecompiledSource {
            contents: None,
            path: std::ffi::OsString::new(),
        };

        let output = self.request_dist_compile_from_stdout(&addr, &compiler_input, &precompiled_source).await;
        log::debug!("request dist compile with precompiled source files elapsed time {:?}", now.elapsed());
        return output;
    }

    //TODO: should remove 'from_stdout'
    async fn request_dist_compile_from_stdout(&self, addr: &str, input: &CompilerInput, precompiled: &PrecompiledSource) -> CompilerOutput {
    
        let cversion = parse_version_from_path(input.compiler_path.as_os_str().to_str().unwrap()).unwrap();
        log::debug!("in commands compiler version: {:?}, addr: {:?}", cversion, addr);
        if self.sender.lock().unwrap().check(addr, &cversion) {
            
            let output = request_dist_compile_with_precompiled_source(addr, &input, &precompiled, &self.runtime).await;
            if output.status == 0 {
            
            }
            else {
                log::trace!("request remote compile and sync back failed: {:?} {:?}", output.out, output.err);
            }
            return output;
        }
        else {
            log::error!("dist compile failed. addr: {} no available remote compiler {:?}", addr, cversion);
            return CompilerOutput::default();
        }
    }
    
    async fn load_and_transmit_precompiled_result(&self, source_files: &Vec<String>, addr: &str, project_name: &std::ffi::OsString, precompiled_result: &PrecompiledResult) -> Vec<std::ffi::OsString> {
        
        let mut source_files = source_files.to_owned();

        let mut handles = Vec::new();
        loop {
            let mut left = Vec::<String>::new();
            if source_files.len() >= 32 {
                left = source_files.split_off(32);
            }

            let project_name = project_name.clone();
            let object = precompiled_result.clone();
            let addr = addr.to_owned();
            let runtime = self.runtime.clone();
            let bulk_handle = self.runtime.spawn(async move {
                let files = Vec::new();
                //files = transmit_precompiled_source_file(&addr, &project_name, object, &source_files.clone(), &runtime).await;
                return files;
            });

            handles.push(bulk_handle);

            if left.is_empty() {
                break;
            }

            source_files = left;
        }

        let mut precompiled_files = Vec::<std::ffi::OsString>::new();
        for handle in handles {
            let mut files = handle.await.unwrap();
            precompiled_files.append(&mut files);
        }

        return precompiled_files;
    }
    
    fn merge_sdk_includes_into_commands(&self, compiler_commands: &Vec<std::ffi::OsString>) -> Vec<std::ffi::OsString> {
        let mut commands = compiler_commands.clone();

        for path in &self.work_env.winkits_includes_path {
            if !commands.contains(&path) {
                commands.push(std::ffi::OsString::from("/I"));
                commands.push(std::ffi::OsString::from(format!("{}", path.to_string_lossy())));
            }
        }

        if !commands.contains(&self.work_env.msvc_includes_path.clone().into_os_string()) {
            commands.push(std::ffi::OsString::from("/I"));
            commands.push(std::ffi::OsString::from(format!("{}", self.work_env.msvc_includes_path.to_string_lossy())));
        }

        return commands;
    }
    
    async fn handle_compile_by_filestream(&self, fileout: &mut FileOut, compiler_input: &CompilerInput, addr: &str, compiler_commands: &Vec<std::ffi::OsString>) 
        -> CompilerOutput {

        let actions = parse_action_from_commands(&std::ffi::OsString::from("MSBuild"), compiler_commands, &compiler_input.compiler_working_dir);

        while let Some(line) = fileout.out.next() {
            log::debug!("precompile stdout: {:?}", line);
        }

        let mut handles = Vec::new();
        let (stream, notify) = crate::communicate::distributor::Distributor::archive_stream(&addr, &self.runtime).await;

        while let Some(line) = fileout.err.next() {
            log::debug!("precompile stderr: {:?}", line);

            let file = line.unwrap();
            if file.contains("Generating Code...") || file.contains("Compiling...")
            {
    
            }
            else {
                let runtime = self.runtime.clone();
                let precompiled_result_path = actions.precompiled_result_file.clone();
                let stream = stream.clone();
                let compiler_input_ = compiler_input.clone();
                let handle = self.runtime.spawn(async move {
                    let files = transmit_precompiled_source_file(&compiler_input_, stream, precompiled_result_path.clone(), &vec![file], &runtime).await;
                    return files;
                });

                handles.push(handle);
            }
        }
        
        if handles.is_empty() {
            log::warn!("no precompiled source file found in precompile output.");
            let mut out = CompilerOutput::default();
            out.status = 1;
            return out;
        }
        else
        {
            let mut files = Vec::new();
            for handle in handles {
                let mut files_ = handle.await.unwrap();
                files.append(&mut files_);
            }

            drop(stream);
    
            //TODO: should be use ref of compiler_input.
            let mut compiler_input = compiler_input.clone();
            let mut commands = tidyup_commands_for_precompile(compiler_commands);
            commands.append(&mut files);
            compiler_input.compiler_commands = commands;
    
            notify.notified().await;

            let output = self.request_dist_compile_with_command(&addr, compiler_input.clone()).await;
            return output;
        }
    }
    
}


async fn handle_compile_by_stdstream(mut stdout: StdOut) {

    let file: std::sync::Arc<Vec<u8>>;

    while let Some(line) = stdout.out.next_line().await.unwrap() {
        println!("stdout: {}", line);
        //#line 1 "E:\\TestFuture\\GammaRay\\GammaRayTool\\tests\\executiontest.cpp"
        if line.starts_with("#line 1 ") {
 
        }
    }

    while let Some(line) = stdout.err.next_line().await.unwrap() {
        println!("stderr: {}", line);
    }

    let status = stdout.child.wait().await.unwrap();
    let code = status.code();
    println!("status code: {:?}", code);
    /*
    let mut content = String::from_utf8_lossy(&stdout);
    //TODO should not convrt to String, mybe operate stdout direct.

    let mut index = 0;
    let mut handles = Vec::new();

    let addr_ = addr.clone();

    for file in &files {
        
        let mut precompiled_result_path = std::ffi::OsString::from(file);
        match actions.precompiled_result_file.clone() {
            PrecompiledResult::PathWithPCResultName(path) => {
                precompiled_result_path = path.into_os_string();
            },
            PrecompiledResult::PathWithoutPCResultName(path) => {
                let path = path.join(file);
                precompiled_result_path = path.into_os_string();
            },
            _ => {},
        }
        commands.push(precompiled_result_path.clone());
        
        if let Some(next_file) = files.get(index + 1) {
            //#line 1 "D:\\TrainSpace\\json\\tests\\abi\\main.cpp"
            let line = format!(r#"#line 1 "{}""#, next_file).replace(r"\", r"\\");
            if let Some(position) = content.find(&line) {
                if position.gt(&0) {

                    let (first, last) = content.split_at(position);
        
                    let msvc_compile_empty_input = CompilerInput {
                        project: project.to_owned(),
                        compiler_path: compiler_path.to_owned(),
                        compiler_working_dir: std::ffi::OsString::from(""),
                        compiler_commands: Vec::<std::ffi::OsString>::new(),
                        build_and_compiler_type: std::ffi::OsString::from("Sync Precompiled Source File")
                    };

                    let precompiled_suorce = PrecompiledSource {
                        contents: Some(first.as_bytes().to_vec()),
                        path: precompiled_result_path.clone()
                    };

                    log::debug!("sync precompiled source file: {:?}. size: {:.2?}M.", std::path::PathBuf::from(precompiled_result_path).file_name().unwrap(), first.as_bytes().len() as f32 / 1024.0 / 1024.0);
                    content = last.to_string().into();

                    let addr__ = addr_.clone();
                    let self_ = self.clone();

                    let handle = self.runtime.spawn_blocking( move || {
                        let _ = self_.request_dist_compile_from_stdout(&addr__,
                            &msvc_compile_empty_input, &precompiled_suorce);
                    });
                    
                    handles.push(handle);
                }
                else {
                    log::warn!("posite source file line in precompile source stdout failed.");
                    break;
                }
            }
        }
        else {

            let compiler_input = CompilerInput {
                project: project.to_owned(),
                compiler_path: compiler_path.to_owned(),
                compiler_working_dir: compiler_working_dir.to_owned(),
                compiler_commands: commands.to_owned(),
                build_and_compiler_type: std::ffi::OsString::from("MSBuild Precompile")
            };

            let precompiled_suorce = PrecompiledSource {
                contents: Some(content.as_bytes().to_vec()),
                path: std::ffi::OsString::from(&precompiled_result_path)
            };

            self.runtime.block_on(async {
                for handle in handles {
                    handle.await.expect("join sync precompiled results thread failed.");
                }    
            });
            
            let now = std::time::Instant::now();
            let addr__ = addr_.clone();
            
            let cooutput = self.runtime.block_on(async move {
                let output = self.request_dist_compile_from_stdout(&addr__, &compiler_input, &precompiled_suorce).await;
                return output;
            });
            
            log::debug!("request {:?} dist compile with precompiled source size: {:?}M, response elapsed: {:?}.", std::path::PathBuf::from(precompiled_result_path).file_name().unwrap(), content.as_bytes().len() as f32 / 1024.0 / 1024.0, now.elapsed());
    
            output.status = cooutput.status;
            output.out = cooutput.out;
            output.err = cooutput.err;
            break;
        }
        index = index.add(1);
    }

    self.sender.lock().unwrap().done(addr.as_str());
    
     */
}   

async fn transmit_precompiled_source_file(compiler_input: &CompilerInput, stream: Option<tokio::sync::mpsc::Sender<crate::communicate::package::ArchiveArgs<'static>>>, precompiled_result: PrecompiledResult, source_files: &Vec<String>, runtime: &std::sync::Arc<tokio::runtime::Handle>)-> Vec<std::ffi::OsString> {

    let now = std::time::Instant::now();
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Zstd);
    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut cursor);

    let mut precompiled_files_path = std::vec::Vec::<std::ffi::OsString>::new();
    let mut intermediate = std::path::PathBuf::new();

    for file in source_files.to_owned() {

        match &precompiled_result {
            PrecompiledResult::PathWithPCResultName(path) => {
                intermediate = path.to_owned();
                intermediate.set_extension("i");
            },
            PrecompiledResult::PathWithoutPCResultName(path) => {
                intermediate = path.join(file);
                intermediate.set_extension("i");
            },
            PrecompiledResult::PCResultNameWithoutPath(_path) => {

            }
            _ => {},
        }

        log::debug!("zip precompiled file: {:?}", intermediate);

        //TODO use tokio file io
        if let Ok(file) = std::fs::File::open(&intermediate) {
            zip.start_file(intermediate.file_name().unwrap().to_string_lossy(), options.to_owned()).unwrap();
            let mut file = std::io::BufReader::new(file);
            let _ = std::io::copy(&mut file, &mut zip);

            precompiled_files_path.push(intermediate.clone().into_os_string());
        }
        else {
            log::error!("precompiled file not found: {:?}", intermediate);   
        }
    }
    log::debug!("zip precompiled file count {} {:?} elapsed time: {:?}", source_files.len(), intermediate, now.elapsed());

    let content = zip.finish().unwrap();
    let file = content.to_owned().into_inner();
    let content = std::borrow::Cow::from(file);

    if intermediate.is_file() {
        intermediate = intermediate.parent().map_or_else(|| std::path::PathBuf::from(""), |parent| parent.to_path_buf());
        intermediate.set_extension("zip");
    }   
    else {
        intermediate.set_extension("zip");
    }

    let file = crate::communicate::package::ArchiveArgs {
        file_type:  crate::communicate::package::FileType::PrecompiledSrcFiles,
        name: source_files.join(",").into(),
        project: compiler_input.project.to_string_lossy().to_string(),
        path: intermediate.to_string_lossy().to_string(),    
        content: content.clone(),
    };

    if let Some(stream) = stream {
        let _ = stream.send(file).await.unwrap_or_else(|err| {
            log::error!("send precompiled source file to stream error: {} file: {:?}", err, intermediate.to_string_lossy());
        });
    }

    /* 
        let intput = CompilerInput {
        project: project_name.into(),
        ..Default::default()
        };

        let _ = crate::communicate::distributor::Distributor::compile(&addr, intermediate.as_os_str().into(), &intput, &content, runtime).await;
    */

    log::debug!("transmit precompiled source file count {} {:?} elapsed time {:?}", source_files.len(), intermediate, now.elapsed());

    return precompiled_files_path;
}

fn request_local_precompile(compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
        compiler_commands: &Vec<std::ffi::OsString>, by_stdout: bool) -> std::io::Result<OutType> {
    
    let mut commands = compiler_commands.to_owned();
    if by_stdout {
        commands.insert(0, std::ffi::OsString::from(r"/E"));
        //remove '/MP'. /E incompatible with multiprocessing
        commands.retain(|item| !item.to_string_lossy().starts_with("/MP"));

        match start_local_compiler_by_stream(compiler_path, compiler_working_dir, &commands) {
            Ok((out, err, child)) => {
                let std = crate::compiler::msvc::StdOut {
                    out: out,
                    err: err,
                    child: child,
                };

                return Ok(OutType::Std(std));
            },
            Err(err) => {
                let error = format!("spawn compile child process error:: {}", err);
                log::warn!("{}", error);
                return Err(err);
            }
        }
    }
    else {
        commands.insert(0, std::ffi::OsString::from(r"/P"));

        if let Some(arg) = compiler_commands.iter().find(|arg| {
            let arg = arg.to_string_lossy();
            return arg.starts_with("/Fo") && (arg.ends_with(".obj") || arg.ends_with("\\"));
            }) {
            let path = arg.to_string_lossy().replace("/Fo", "/Fi").replace(".obj", ".i").replace("\\\\", "\\");
            commands.insert(1, std::ffi::OsString::from(path));
        }

        match start_local_compiler_by_file(compiler_path, compiler_working_dir, &commands) {
            Ok((stdout, stderr, child)) => {
                let file = crate::compiler::msvc::FileOut {
                    out: stdout,
                    err: stderr,
                    child: child,
                };
                return Ok(OutType::File(file));
            },
            Err(err) => {
                let error = format!("spawn compile child process error:: {}", err);
                log::warn!("{}", error);
                return Err(err);
            }
        }
    }
}

fn push_project_name_to_precompiled_file_path(path: &std::path::PathBuf, obejct: String) -> std::path::PathBuf
{
    if !obejct.is_empty() && obejct.contains(".dir") {
        let index = obejct.find(".dir").unwrap();
        let (first, _)= obejct.split_at(index);
        let (_, obejct_name) = first.split_at(3);
        
        let mut path = path.to_owned();
        path.set_extension("i");
        if let Some(precompile_file_name) = path.file_name() {
            if let Some(parent) = path.parent() {
                let mut path = parent.to_path_buf();
                path.push(obejct_name);
                path.push(precompile_file_name);
                return path;
            }
            else {
                return path;
            }
        }
        else {
            return path;
        }
    }
    else {
        return path.to_path_buf();
    }
}

fn parse_version_from_path(path: &str) -> Option<crate::replica::toolchain::CompilerVersion> {
    let path = std::path::PathBuf::from(path);

    let mut iter = path.components().skip_while(|&item| {
            let item = item.as_os_str().to_string_lossy();
            let vec = item.split('.').collect::<Vec<&str>>();
            if vec.len() >= 3 {
                return false;
            }
            else {
                return true;                
            }
    });
    
    if let Some(version) = iter.next() {
        
        let mut cversion = crate::replica::toolchain::CompilerVersion {
            version: version.as_os_str().to_string_lossy().to_string(),
            host: crate::replica::toolchain::Arch::unknown,
            target: crate::replica::toolchain::Arch::unknown,
        };
        
        iter.next();

        if let Some(host) = iter.next() {
            let host = host.as_os_str().to_str().unwrap();
            let host = crate::replica::toolchain::Arch::format(host);
            cversion.host = host;
        } 
        
        if let Some(target) = iter.next() {
            let target = target.as_os_str().to_str().unwrap();
            let target = crate::replica::toolchain::Arch::format(target);
            cversion.target = target;
        }
        
        return Some(cversion);
    }
    else {
        return None;
    }

    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
}

fn request_local_compile(compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
                                compiler_commands: &Vec<std::ffi::OsString>, build_and_compiler_type: std::ffi::OsString,
                            sync_compile_result: bool) -> (CompilerOutput, Option<CompiledResults>) {
    let now = std::time::Instant::now();
    let (status, stdout, stderr) = start_local_compiler(&compiler_path, &compiler_working_dir, &compiler_commands);
    let compile_output = String::from_utf8_lossy(&stdout);
    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    let mut compiled_results: CompiledResults = Vec::new();

    if status == 0 {
        let lines = compile_output.lines().collect();

        let (files, _wraning_or_message) = filter_compiler_warning_and_error(lines);

        let mut last = files.clone().last().cloned();

        if let Some(index) = files.iter().rposition(|&arg| arg.ends_with(".i")) {
            let &arg = files.index(index);
            last = Some(arg);
        }
        log::trace!("last compiled source file: {:?}", last);
        log::debug!("local compile file count: {:?}, elapsed: {:?}.", files.len(), now.elapsed());

        let (pdb_path, _one_pdb )= fetch_compile_pdb_path(build_and_compiler_type.clone(), compiler_commands.to_owned(), compiler_working_dir);
        let pdb_path = std::rc::Rc::new(pdb_path);
        for line in files {
            let line = line.replace(r#"""#, "");
            if line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".cc") || line.ends_with(".i") {
                if sync_compile_result {
                    let mut obj: Option<(std::ffi::OsString, Vec<u8>)> = None;
                    let mut pdb: Option<(std::ffi::OsString, Vec<u8>)> = None;
                    let mut idb: Option<(std::ffi::OsString, Vec<u8>)> = None;
    
                    let mut result_path = std::path::PathBuf::from("");
                    let object = fetch_compile_object_file(&build_and_compiler_type, compiler_commands, compiler_working_dir);
                    match object {
                        GeneratedObject::PathWithObjName(path) => {
                            result_path = path;
                        },
                        GeneratedObject::PathWithoutObjName(dir) => {
                            let path = dir.join(&line);
                            result_path = path;
                            result_path.set_extension("obj");
                        },
                        _ => {
                            log::warn!("fetch result file path failed.");
                        }
                    };
                    
                    match std::fs::File::open(&result_path) {
                        Ok(file) => {
                            let mut contents = Vec::new();
                            let mut file = std::io::BufReader::new(file);
                            let _ = file.read_to_end(&mut contents).unwrap();
                            obj = Some((std::ffi::OsString::from(result_path.to_str().unwrap()), contents));
                        },
                        Err(error) => {
                            if error.kind() == std::io::ErrorKind::NotFound {
                                log::trace!("obj file path is not found.");
                            }
                            else {
                                log::trace!("obj file read failed. {:?}", error);
                            }
                        }
                    };
    
                    result_path.clear();

                    if last == Some(&line)
                    {
                        let pdb_path = pdb_path.clone();
                        match &*pdb_path {
                            ProgramDataBase::PathWithPDBName(path) => {
                                result_path = path.clone();
                            },
                            ProgramDataBase::PathWithoutPDBName(dir) => {
                                result_path = dir.join(&line);
                                result_path.set_extension("pdb");
                                log::trace!("generate program database path without obj name, {:?}", result_path);
                            },
                            _ => {
                                log::warn!("fetch result file path failed.");
                            }
                        };
                        
                        match std::fs::File::open(&result_path) {
                            Ok(file) => {
                                let mut contents = Vec::new();
                                let mut file = std::io::BufReader::new(file);
                                let _ = file.read_to_end(&mut contents).unwrap();
                                pdb = Some((std::ffi::OsString::from(result_path.to_str().unwrap()), contents));
                            },
                            Err(error) => {
                                if error.kind() == std::io::ErrorKind::NotFound {
                                    log::trace!(".pdb file path is not found.");
                                }
                                else {
                                    log::warn!(".pdb file read failed. {:?}", error);
                                }
                            }
                        }

                        result_path.set_extension("idb");
                        match std::fs::File::open(&result_path) {
                            Ok(file) => {
                                let mut contents = Vec::new();
                                let mut file = std::io::BufReader::new(file);
                                let _ = file.read_to_end(&mut contents).unwrap();
                                idb = Some((std::ffi::OsString::from(result_path.to_str().unwrap()), contents));
                            },
                            Err(error) => {
                                if error.kind() == std::io::ErrorKind::NotFound {
                                    log::trace!(".idb file path is not found.");
                                }
                                else {
                                    log::warn!(".idb file read failed. {:?}", error);
                                }
                            },
                        }
                    }
                    //TODO: should be add warning or error or other message.
                    let processed_result = crate::compiler::model::CompiledResult {
                        source_file: std::ffi::OsString::from(&line),
                        obj: obj,
                        pdb: pdb,
                        idb: idb,
                    };
                    compiled_results.push(processed_result);
                }

                compiled_filename.push(std::ffi::OsString::from(line));
            }
            else {
                log::trace!("exclude source file,maybe warning and error. {:?}", line);
            }
        }
    }
    else {
        
    }
    
    let output = CompilerOutput {
        status: status,
        out: stdout,
        err: stderr,
    };

    return (output, Some(compiled_results));
}

fn request_local_compile_by_preprocessed_source(msvc_compile_input: &CompilerInput, _pool: std::sync::Arc<tokio::runtime::Handle>) -> (CompilerOutput, Option<CompiledResults>) {

    //replace .cpp/.c/.cc to .i
    let commands = msvc_compile_input.compiler_commands.clone();

    let obj_file:Vec<std::ffi::OsString> = commands.clone().into_iter().filter(|value| value.to_string_lossy().starts_with("/Fo")).collect();
    if !obj_file.is_empty() {
        let mut obj_file_path = obj_file.first().unwrap().to_string_lossy().to_string();
        obj_file_path = obj_file_path.replace("/Fo", "");
        obj_file_path = obj_file_path.replace(r#"\\"#, r"\");
        let path = std::path::PathBuf::from(&msvc_compile_input.compiler_working_dir).join(&obj_file_path);
        if !path.exists() {
            match std::fs::create_dir_all(&path) {
                Ok(_) => {},
                Err(error) => {
                    log::debug!("create .pdb dir {:?}, error {:?}", path, error);
                },
            }
        }
    }

    let pdb_file:Vec<std::ffi::OsString> = commands.clone().into_iter().filter(|value| value.to_string_lossy().starts_with("/Fd")).collect();
    if !pdb_file.is_empty() {
        let mut pdb_file_path = pdb_file.first().unwrap().to_string_lossy().to_string();
        pdb_file_path = pdb_file_path.replace("/Fd", "");
        pdb_file_path = pdb_file_path.replace(r#"\\"#, r"\");

        let path = std::path::PathBuf::from(&pdb_file_path);
        if path.has_root() {
            if path.extension() == Some(&std::ffi::OsString::from("pdb")) {
                let path = path.parent().unwrap();
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
            }
            else {
                let _ = std::fs::create_dir_all(&path);
            }
        }
        else {
            let path = std::path::PathBuf::from(&msvc_compile_input.compiler_working_dir).join(&path);
            if path.extension() == Some(&std::ffi::OsString::from("pdb")) {
                let path = path.parent().unwrap();
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
            }
            
            else {
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
            }
        }
    }

    if !msvc_compile_input.compiler_working_dir.is_empty() {
        let path = std::path::PathBuf::from(&msvc_compile_input.compiler_working_dir);
        if !path.exists() {
            match std::fs::create_dir_all(&path) {
                Ok(_) => {},
                Err(error) => {
                    log::warn!("dist worker create dir {:?} failed. {:?}.", &path, error);
                },
            }
        }
    }

    let (output, results) = request_local_compile(&msvc_compile_input.compiler_path,
                    &msvc_compile_input.compiler_working_dir, &commands,
                    msvc_compile_input.build_and_compiler_type.clone(), true);

    return (output, results);
}

fn request_dist_compile_with_source_and_include(working_param: &crate::platform::windows::WindowsCompilerEnv, msvc_compile_input: &CompilerInput) -> CompilerOutput {
    log::debug!("request dist compile with source and include file.");
    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
    let local_compiler_arch = msvc_compile_input.compiler_path.to_str().unwrap();
    let compiler_dir = std::path::Path::new(&working_param.compiler_path).join("Hostx64").join(local_compiler_arch);

    log::trace!("vs compiler install dir: {:?}.", compiler_dir);

    let winsdk_path = working_param.winkits_includes_path.first().unwrap();

    let mut dist_msvc_compiler_path: std::ffi::OsString = std::ffi::OsString::from("");
    let dist_msvc_include_path: std::ffi::OsString = std::ffi::OsString::from("");
    let win_kits_include_dir: std::ffi::OsString = std::ffi::OsString::from("");

    //(dist_msvc_compiler_path, dist_msvc_include_path, win_kits_include_dir) = sender.dist_kits_and_tool_pre_sync(winsdk_path, compiler_dir.to_str().unwrap());

    if dist_msvc_compiler_path.is_empty() {
        log::debug!("msvc toolchain sync");
        let path = std::path::PathBuf::from(compiler_dir.to_str().unwrap());
        if path.is_dir() {
            //(dist_msvc_compiler_path, dist_msvc_include_path) = sender.sync_toolchain(&path);
        }
    }
    else {
        log::debug!("tool chain sync have done");
    }

    if win_kits_include_dir.is_empty() {
        log::debug!("windows kits sync");
        let mut path = std::path::PathBuf::from(winsdk_path);
        if path.is_dir() && path.pop() {
            //win_kits_include_dir = sender.sync_windows_kits(&path);
        }
    }
    else {
        log::debug!("windows kits sync have done");
    }

    if !dist_msvc_compiler_path.is_empty() {
        let mut path = std::path::PathBuf::from(dist_msvc_compiler_path);
        if !path.ends_with("cl.exe") {
            path.push("cl.exe")
        }
        dist_msvc_compiler_path = std::ffi::OsString::from(path);
    }

    let env_input = crate::platform::windows::WindowsCompilerEnv {
         winkits_includes_path: vec![win_kits_include_dir],
         compiler_path: std::path::PathBuf::from(dist_msvc_compiler_path.clone()),
         msvc_includes_path: std::path::PathBuf::from(dist_msvc_include_path),
         msvc_version: String::new(),
         env_args: String::new(),
    };

    let mut input = msvc_compile_input.to_owned();
    //input.env_input = Some(env_input.clone());
    input.build_and_compiler_type = std::ffi::OsString::from("MSBuild Dist");

    return CompilerOutput::default();
}

async fn request_dist_compile_with_precompiled_source(addr: &str, input: &CompilerInput, precompiled: &PrecompiledSource, runtime: &std::sync::Arc<tokio::runtime::Handle>) -> CompilerOutput {

    let mut output = CompilerOutput::default();

    let now = std::time::Instant::now();
    let path = precompiled.path.clone();
    if !input.compiler_commands.is_empty() || !precompiled.contents.is_some() {
        if let Some(content) = precompiled.contents.clone() {
            log::info!("precompiled sourcefile result has content. so just transmit file");
            let content = std::borrow::Cow::from(content);
            let receiver = crate::communicate::distributor::Distributor::compile(addr, path, input, &content, runtime).await;
            match receiver {
                crate::communicate::package::ReceiverType::Archive(recv) => {
                    output.status = recv.status as u32;
                },
                _ => {
                    log::error!("communicate compile error.");
                }   
            }
        }
        else {
            log::info!("precompiled sourcefile result content is empty. so just transmit command"); 
            let receiver = crate::communicate::distributor::Distributor::compile(addr, path, input, &std::borrow::Cow::from(Vec::new()), runtime).await;
            match receiver {
                crate::communicate::package::ReceiverType::Compile(recv) => {
                    output.status = recv.status;
                    output.out = std::sync::Arc::new(recv.out);
                    output.err = std::sync::Arc::new(recv.err);
                }
                _ => {
                    log::error!("communicate compile error.");
                }
            }
        }   
    }
    else {
        log::warn!("compiler commands and context is all empty, so do nothing.")
    }

    log::debug!("communicate distribute compile elapsed: {:?}", now.elapsed());

    return output;
}

fn start_local_compiler(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    use std::process::Stdio;

    log::trace!("local compile working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compiler commands: {:?}", compiler_commands);
    
    let start = std::time::Instant::now();
    let mut process = std::process::Command::new(compiler_path);

    for item in compiler_commands {
        if item.to_string_lossy().contains(" ") {
            process.arg(item);    
        }
        else {
            use std::os::windows::process::CommandExt;
            process.raw_arg(item);
        }
    }
    
    let child = process.current_dir(working_dir)    
                            //.args(compiler_commands)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn();
    
    match child {
        Ok(child) => {
            match child.wait_with_output() {
                Ok(output) => {
                    if output.status.success() {
                        //if precompile
                        //files warning error form stdout
                        //logo message form stderr

                        let elapsed = start.elapsed();
                        let output_context = String::from_utf8_lossy(&output.stdout);
                        
                        let stderr_context = String::from_utf8_lossy(&output.stderr); //filename

                        let lines:Vec<&str> = stderr_context.lines().collect();
                        let (files, warning_or_message) = filter_compiler_warning_and_error(lines);
                        
                        log::trace!("compile local file count {:?} success, elapsed time: {:?}, file: {:?}, warning: {:?}, stderr: {:?}", files.len(), elapsed, files, warning_or_message, stderr_context);
                
                        return (output.status.code().expect("process exit code unwrap failed.") as u32, std::sync::Arc::new(output.stdout), std::sync::Arc::new(output.stderr));
                        //TODO shoud not be used Arc wrap
                    }
                    else {
                        let output_context = String::from_utf8_lossy(&output.stdout);  //error
                        let lines:Vec<&str> = output_context.lines().collect();
                        let (files, error_or_message) = filter_compiler_warning_and_error(lines);

                        let stderr_context = String::from_utf8_lossy(&output.stderr);
                        let elapsed = start.elapsed();
                        log::info!("compile file count {:?} failure, elapsed time: {:?}. file: {:?}, error: {:?}, error code: {:?}, stderr:  {:?}", files.len(), elapsed, files, error_or_message, output.status.code(), stderr_context);

                        return (output.status.code().expect("process exit code unwrap failed.") as u32, std::sync::Arc::new(output.stdout), std::sync::Arc::new(output.stderr));
                    }
                },
                Err(error) => {
                    log::warn!("compile child wait output error: {:?}", error);
                    let mut error_description = String::from("compile child wait output error: ");
                    error_description.push_str(error.to_string().as_str());

                    return (1001, std::sync::Arc::new(error_description.into_bytes()), std::sync::Arc::new(vec![]));
                },
            }
        },
        Err(error) => {
            
            println!("spawn compile child process error: {:?}", error);
            let mut error_description = String::from("spawn compile child process error: ");
            error_description.push_str(error.to_string().as_str());
            return (1002, std::sync::Arc::new(error_description.into_bytes()), std::sync::Arc::new(vec![]));
        }
    }
}

type OutStreamByFile = Box<std::io::Lines<std::io::BufReader<std::process::ChildStdout>>>;
type ErrStreamByFile = Box<std::io::Lines<std::io::BufReader<std::process::ChildStderr>>>;

fn start_local_compiler_by_file(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) 
    -> std::io::Result<(OutStreamByFile, ErrStreamByFile, std::process::Child)> {
    
    use std::process::Stdio;

    log::trace!("local compile working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compiler commands: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    let mut process = std::process::Command::new(compiler_path);

    for item in compiler_commands {
        if item.to_string_lossy().contains(" ") {
            process.arg(item);    
        }
        else {
            use std::os::windows::process::CommandExt;
            process.raw_arg(item);
        }
    }

    let child = process.current_dir(working_dir)    
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn();
    
    match child {
        Ok(mut child) => {
            let elapsed = start.elapsed();
            log::trace!("compile local file stream elapsed time: {:?}", elapsed);

            let stdout = child.stdout.take().unwrap();
            let reader = std::io::BufReader::new(stdout);
            let outstream = Box::new(reader.lines());

            let stderr = child.stderr.take().unwrap();
            let reader = std::io::BufReader::new(stderr);
            let errstream = Box::new(reader.lines());

            return Ok((outstream, errstream, child));
        },
        Err(error) => {
            log::warn!("spawn compile child process error: {:?}", error);
            let mut error_description = String::from("spawn compile child process error: ");
            error_description.push_str(error.to_string().as_str());
            return Err(error);
        }
    }
}

type OutStream = Box<tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>>;
type ErrStream = Box<tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStderr>>>;
fn start_local_compiler_by_stream(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> 
    std::io::Result<(OutStream, ErrStream, tokio::process::Child)>
{
    use std::process::Stdio;

    log::trace!("working dir: {:?}", working_dir);
    log::trace!("path: {:?}", compiler_path);
    log::trace!("commands: {:?}", compiler_commands);
    
    let start = std::time::Instant::now();
    let child = tokio::process::Command::new(compiler_path)
                            .current_dir(working_dir)
                            .args(compiler_commands.clone())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn();

    match child {
        Ok(mut child) => {
            let elapsed = start.elapsed();
            log::trace!("compile local file stream elapsed time: {:?}", elapsed);

            use tokio::io::AsyncBufReadExt;

            let stdout = child.stdout.take().unwrap();
            let reader = tokio::io::BufReader::new(stdout);
            let outstream = Box::new(reader.lines());


            let stderr = child.stderr.take().unwrap();
            let reader = tokio::io::BufReader::new(stderr);
            let errstream = Box::new(reader.lines());

            return Ok((outstream, errstream, child));
        },
        Err(error) => {
            log::warn!("spawn compile child process error: {:?}", error);
            return Err(error);
        }
    }
}

fn extract_additional_input_commands(compiler_commands: &mut String) -> Option<std::ffi::OsString> {

    let compiler_path_begin_position = compiler_commands.find(" AssistClCompilerPath:");
    match compiler_path_begin_position {
        Some(index) => {
            let mut half_args:String = compiler_commands.drain(..index).collect();
            let compiler_path_end_position = compiler_commands.find("cl.exe");
            match compiler_path_end_position {
                Some(index) => {
                    let mut cl_path:String = compiler_commands.drain(..index + "cl.exe".len()).collect();
                    let _ = cl_path.drain(.." AssistClCompilerPath:".len());

                    half_args += &compiler_commands;

                    compiler_commands.clear();
                    *compiler_commands = half_args;
                    return Some(std::ffi::OsString::from(cl_path));
                },
                _ => {
                    return None;
                },
            };
        },
        _ => {
            return None;
        },
    };
}

// /D "CMAKE_INTDIR=\"Release\"" 
fn extract_macro_contain_space_arg(compiler_commands: &mut String) -> Vec<std::ffi::OsString> {

    let mut args: Vec<std::ffi::OsString> = Vec::new();
    let replace_regex = regex::Regex::new(r#"(?P<a>/[Dd])(?P<b>[\s])(?P<c>"[\w=\\]+"[\w\\]+"")"#).unwrap();
    for capture in replace_regex.captures_iter(&compiler_commands.clone())
    {   
        let macro_definition = capture.index(0);
        let macro_definition_without_space = macro_definition.replace(" ", "");
        args.push(std::ffi::OsString::from(macro_definition_without_space));

        let mut macro_definitin_with_space = macro_definition.to_owned();
        macro_definitin_with_space.push_str(" ");

        let commands = compiler_commands.replace(macro_definitin_with_space.as_str(), "");
        *compiler_commands = String::from(commands);

    }
    return args;
}

fn extract_path_ecnclosed_quotation_arg(compiler_commands: &mut String) -> Vec<std::ffi::OsString> {

    let pathwithspace_regex = regex::Regex::new(r#"[-/\w:]*".*?""#).unwrap();
    let mut include_args: Vec<std::ffi::OsString> = Vec::new();

    for capture in pathwithspace_regex.captures_iter(&compiler_commands.clone())
    {
        if capture.len() == 1 {
            let path_contain_space = capture.index(0);

            let pathwithspace_position = compiler_commands.find(path_contain_space);
            match pathwithspace_position {
                Some(index) => {
                    let (head_content, _) = compiler_commands.split_at(index);
                    if head_content.len() > "external:I ".len() {
                        let prefix_index = head_content.len() - "external:I ".len();
                        let prefix_arg = head_content.to_owned().split_off(prefix_index);
                        if prefix_arg.eq("external:I ") {
                            let path_without_quotation = path_contain_space.replace('"', "");
                            let include_arg = "/I".to_string() + &path_without_quotation;
                            include_args.push(std::ffi::OsString::from(include_arg));

                        }
                    }
                },
                _ => {},
            }

            if path_contain_space.contains(" ") || 
                (path_contain_space.starts_with(r#"/I""#) && path_contain_space.ends_with(r#"""#)) ||
                path_contain_space.contains(".cpp") || path_contain_space.contains(".c") ||  path_contain_space.contains(".cc") {
                let path_without_quotation = path_contain_space.replace('"', "");
                include_args.push(std::ffi::OsString::from(path_without_quotation));
                let arg = String::from(" ") + path_contain_space;
                *compiler_commands = String::from(compiler_commands.replace(arg.as_str(), ""));
            }
            else {

            }
        }
        else 
        {
            println!("regex capture len is not 1.");
        }
    }
    return include_args;
}

fn split_commonds_by_space(compiler_commands: &mut String) -> Vec<std::ffi::OsString> {
    log::trace!("compiler commands {:?}", compiler_commands);
    let compiler_commands = compiler_commands.replace("  ", " ");
    let args_vec = compiler_commands.split(" ").map(|arg| String::from(arg)).collect::<Vec<_>>();
    let mut args: Vec<std::ffi::OsString> = Vec::new();

    for arg in args_vec {
        if arg.starts_with("/Fo") || arg.starts_with("/Fd") {
            let arg_without_enclosed_quotation = arg.replace('"', "");
            args.push(std::ffi::OsString::from(arg_without_enclosed_quotation));
        }
        else if !arg.is_empty() {
            args.push(std::ffi::OsString::from(arg));
        }
    }
    
    return args;
}

fn parse_compiler_input_command(compiler_input: CompilerInput, working_compiler_env: &crate::platform::windows::WindowsCompilerEnv) -> (Vec<std::ffi::OsString>, std::ffi::OsString) {

    let compile_commands:Vec<std::ffi::OsString> = compiler_input.compiler_commands;
    let mut args = Vec::new();
    let mut compiler_path = std::ffi::OsString::new();
    let commands_iter = compile_commands.iter().map(|arg| arg.to_str().unwrap().to_owned());
    if compiler_input.build_and_compiler_type.to_string_lossy().contains("MSBuild") {
        let mut commands = commands_iter.last().unwrap();

        args = extract_macro_contain_space_arg(&mut commands);
        let mut args_with_path = extract_path_ecnclosed_quotation_arg(&mut commands);
        args.append(&mut args_with_path);

        let mut args_others = split_commonds_by_space(&mut commands);
        args.append(&mut args_others);

        if args.is_empty() {
            log::warn!("compiler don't effective extract commands.")
        }
        let winkits_includes = &working_compiler_env.winkits_includes_path;
        for include in winkits_includes {
            let mut instruct = "/I".to_string();
            instruct += include.to_str().unwrap();
            args.push(std::ffi::OsString::from(instruct));
        }
    
        let msvc_includes = working_compiler_env.msvc_includes_path.display().to_string();
    
        let mut instruct = String::from("/I");
        instruct += &msvc_includes;
        args.push(std::ffi::OsString::from(instruct));
        
        return (args, compiler_path);
    }
    else if  compiler_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
        compiler_path = compiler_input.compiler_path;
        return (args, compiler_path);
    }
    else {
        return (args, compiler_path);
    }
}

fn fetch_compile_source_file(build_and_compiler_type: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) 
                                    -> Option<std::collections::HashMap<String, std::path::PathBuf>> {

    let working_dir = std::path::PathBuf::from(working_dir);
    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake")
        || build_and_compiler_type.to_string_lossy().contains("Dist") {
        let mut sourcefile: std::collections::HashMap<String, std::path::PathBuf> = std::collections::HashMap::new();
        
        for command in compiler_commands {
            let command = command.to_string_lossy();
            
            if command.to_lowercase().contains(".cpp") || command.to_lowercase().contains(".c") || command.to_lowercase().contains(".cc") {
                let source = command.replace(r#"""#, "");
                let mut index = source.rfind(r"\");
                if index.is_none() {
                    index = source.rfind(r"/");
                }
                match index {
                    Some(i) => {
                        let source_file_name = source.clone().split_off(i + 1);

                        let source_path = std::path::PathBuf::from(source.clone());
                        if source_path.is_absolute() {
                            sourcefile.insert(source_file_name, source_path);
                        }
                        else {
                            let source_path = working_dir.join(source.clone());
                            sourcefile.insert(source_file_name, source_path);
                        }
                    },
                    None => {
                        let absolute_source_path = working_dir.join(source.clone());
                        sourcefile.insert(source.clone(), absolute_source_path);
                    }
                }
            }
        }
        return Some(sourcefile);
    }
    else {

    }

    return None;
}


fn filter_compiler_warning_and_error(lines: Vec<&str>) -> (Vec<&str>, Vec<&str>) {

    let mut files = Vec::new();
    let mut error = Vec::new();

    let mut iter = lines.iter().peekable();

    while let Some(&item) = iter.next() {
        if item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c") || item.ends_with(".cc") {
            if let Some(next) = iter.peek() {
                if next.contains(": error ") {
                   continue;
                }
            }
            files.push(item);
        }
        else {
            error.push(item);
        }
    }

    return (files, error);


    let (files, warning):(Vec<_>, Vec<_>) = lines.into_iter().partition(|item| item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c") || item.ends_with(".cc"));
    return (files, warning);
}

#[derive(Debug)]
struct CompileAction {
    pub precompile_2_stdout: bool,
    pub compile_source_file: std::collections::HashMap<std::string::String, std::path::PathBuf>,
    pub precompiled_result_file: PrecompiledResult,
}

fn parse_action_from_commands(build_and_compiler_type: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) -> CompileAction {

    let working_dir = std::path::PathBuf::from(working_dir);

    let mut precompile_2_stdout = false;
    let mut sourcefile: std::collections::HashMap<String, std::path::PathBuf> = std::collections::HashMap::new();
    let mut precompiled_result_file = PrecompiledResult::NonePCResultPath;
    let mut object_file = GeneratedObject::NoneObjPath;
    let mut pdb_file = ProgramDataBase::NonePDBPath;

    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake")
        || build_and_compiler_type.to_string_lossy().contains("Dist") {

        for command in compiler_commands {
            let mut command = command.to_string_lossy();
            if command == "/P" {
                precompile_2_stdout = false;
            }
            else if command == "/E" {
                precompile_2_stdout = true;
            }
            else if command.to_lowercase().contains(".cpp") || command.to_lowercase().contains(".c") || command.to_lowercase().contains(".cc") {
                let source = command.replace(r#"""#, "");
                let mut index = source.rfind(r"\");
                if index.is_none() {
                    index = source.rfind(r"/");
                }
                match index {
                    Some(i) => {
                        let source_file_name = source.clone().split_off(i + 1);

                        let source_path = std::path::PathBuf::from(source.clone());
                        if source_path.is_absolute() {
                            sourcefile.insert(source_file_name, source_path);
                        }
                        else {
                            let source_path = working_dir.join(source.clone());
                            sourcefile.insert(source_file_name, source_path);
                        }
                    },
                    None => {
                        let absolute_source_path = working_dir.join(source.clone());
                        sourcefile.insert(source.clone(), absolute_source_path);
                    }
                }
            }
            else if command.starts_with("/Fd") {
                (pdb_file, _) = exact_compile_pdb_file(command, &working_dir);
            }
            else if command.starts_with("/Fo") {
                object_file = exact_compile_object_file(command, &working_dir);
            }
            else if command.starts_with("/Fi") {

                let result_path = command.to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
                if result_path.ends_with(".i") {
                    let path = std::path::PathBuf::from(result_path);
                    
                    if path.is_absolute() {
                        precompiled_result_file = PrecompiledResult::PathWithPCResultName(path);
                    }
                    else {
                        let path = working_dir.join(path);
                        precompiled_result_file = PrecompiledResult::PathWithPCResultName(path);
                    }
                }
                else {
                    let path = std::path::PathBuf::from(result_path);
                    if path.is_absolute() {
                        precompiled_result_file = PrecompiledResult::PathWithoutPCResultName(path);
                    }
                    else {
                        let path = working_dir.join(path);
                        precompiled_result_file = PrecompiledResult::PathWithoutPCResultName(path);
                    }
                }
            }
        }

        match precompiled_result_file {
            PrecompiledResult::NonePCResultPath => {
                match object_file {
                    GeneratedObject::PathWithObjName(mut path) => {
                        path.set_extension("i");
                        precompiled_result_file = PrecompiledResult::PathWithPCResultName(path);
                    },
                    GeneratedObject::PathWithoutObjName(path) => {
                        precompiled_result_file = PrecompiledResult::PathWithoutPCResultName(path);
                    },
                    _ => {}
                }
            }
            _ => {},
        }
    }

    return CompileAction {
        precompile_2_stdout,
        compile_source_file: sourcefile,
        precompiled_result_file: precompiled_result_file,
    }

}

fn tidyup_commands_for_precompile(compiler_commands: &Vec<std::ffi::OsString>) -> Vec<std::ffi::OsString> {

    let mut specify_sourcefile_type = true;

    let mut cxx = false;
    let mut c = false;
    let mut commands: Vec<_> = compiler_commands.into_iter().filter(|&item| {
        let item = item.to_string_lossy().to_lowercase();

        if item == "/tp" || item == "/tc" || item == "/TP" || item == "/TC" || item == "/Tc" || item == "/Tp" {
            specify_sourcefile_type = false;
        }

        cxx = item.ends_with(".cpp") || item.ends_with(".cxx") || item.ends_with(".cc");
        c = item.ends_with(".c");

        let removed = item == "/p" || item.starts_with("/fi") ||item == "/P" || item.starts_with("/Fi");
        return !(cxx || c || removed);

    }).map(|item| item.to_owned()).collect();

    if cxx && c {
        log::warn!("cxx/cpp file and c file cannot be specified at the same time");
    }

    if specify_sourcefile_type {
        commands.push(std::ffi::OsString::from(if cxx {"/TP"} else {"/TC"}));
    }

    return commands;
}

enum GeneratedObject {
    NoneObjPath,
    PathWithObjName(std::path::PathBuf),
    PathWithoutObjName(std::path::PathBuf),
}

fn exact_compile_object_file(mut arg: std::borrow::Cow<str>, working_dir: &std::path::PathBuf) -> GeneratedObject {
    let object_path = arg.to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
    if object_path.ends_with(".obj") {
        let path = std::path::PathBuf::from(object_path);
        if path.has_root() {
            return GeneratedObject::PathWithObjName(path);
        }
        else {
            let path = working_dir.join(path);
            return GeneratedObject::PathWithObjName(path);
        }
    }
    else {
        let path = std::path::PathBuf::from(object_path);
        if path.has_root() {
            return GeneratedObject::PathWithoutObjName(path);
        }
        else {
            let path = working_dir.join(path);
            return GeneratedObject::PathWithoutObjName(path);
        }
    }
}

fn fetch_compile_object_file(build_and_compiler_type: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) -> GeneratedObject {

    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake") 
        || build_and_compiler_type.to_string_lossy().contains("Dist") {
        
        let working_dir = std::path::PathBuf::from(working_dir);

        let object_param = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().starts_with("/Fo")).collect::<Vec<_>>();

        match object_param.last() {
            Some(object) => {
                return exact_compile_object_file(object.to_string_lossy(), &working_dir);
            },
            None => {
                log::debug!("NoneObjPath NoneObjPath NoneObjPath");
                return GeneratedObject::NoneObjPath;
            },
        }
        
    }
    return GeneratedObject::NoneObjPath;
}

enum ProgramDataBase {
    NonePDBPath,
    PathWithPDBName(std::path::PathBuf),
    PathWithoutPDBName(std::path::PathBuf),
}

fn exact_compile_pdb_file(mut arg: std::borrow::Cow<str>, working_dir: &std::path::PathBuf) -> (ProgramDataBase, bool) {
    let pdb_path = arg.to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
    let one_pdb = pdb_path.contains(r"\vc14");
    if pdb_path.ends_with(".pdb") {
        let path = std::path::PathBuf::from(pdb_path);
        if path.has_root() {
            return (ProgramDataBase::PathWithPDBName(path), one_pdb);
        }
        else {
            let path = working_dir.join(path);
            return (ProgramDataBase::PathWithPDBName(path), one_pdb);
        }
    }
    else {
        let path = std::path::PathBuf::from(pdb_path);
        if path.has_root() {
            return (ProgramDataBase::PathWithoutPDBName(path), false);
        }
        else {
            let path = working_dir.join(path);
            return (ProgramDataBase::PathWithoutPDBName(path), false);
        }
    }
}

fn fetch_compile_pdb_path(build_and_compiler_type: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) -> (ProgramDataBase, bool) {
    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake") 
        || build_and_compiler_type.to_string_lossy().contains("Dist") {
        
        let working_dir = std::path::PathBuf::from(working_dir);

        let pdb_param = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().starts_with("/Fd")).collect::<Vec<_>>();
        
        //"/FdD:\\TrainSpace\\json\\Build\\tests\\abi\\diag\\Debug\\abi_compat_diag_on.pdb"
        match pdb_param.last() {
            Some(pdb) => {
                return exact_compile_pdb_file(pdb.to_string_lossy(), &working_dir);
            },
            None => {
                println!("do not fetch program database.");
                return (ProgramDataBase::NonePDBPath, false);
            },
        }
    }
    return (ProgramDataBase::NonePDBPath, false);
}

#[derive(Clone, Debug)]
enum PrecompiledResult {
    NonePCResultPath,
    PathWithPCResultName(std::path::PathBuf),
    PathWithoutPCResultName(std::path::PathBuf),
    PCResultNameWithoutPath(std::path::PathBuf),
}

fn fetch_precompiled_result_file(build_and_compiler_type: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) -> PrecompiledResult {
    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake") 
        || build_and_compiler_type.to_string_lossy().contains("Dist") {
        
        let working_dir = std::path::PathBuf::from(working_dir);

        let precompiled_result_param = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().starts_with("/Fi")).collect::<Vec<_>>();

        match precompiled_result_param.last() {
            Some(result) => {
                let result_path = result.to_string_lossy().to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
                if result_path.ends_with(".i") {
                    let path = std::path::PathBuf::from(&result_path);
                    if path.has_root() {
                        return PrecompiledResult::PathWithPCResultName(path);
                    }
                    else {
                        let path = working_dir.join(path);
                        return PrecompiledResult::PathWithPCResultName(path);
                    }
                }
                else {
                    let path = std::path::PathBuf::from(result_path);
                    if path.has_root() {
                        return PrecompiledResult::PathWithoutPCResultName(path);
                    }
                    else {
                        let path = working_dir.join(path);
                        return PrecompiledResult::PathWithoutPCResultName(path);
                    }
                }
            },
            None => {
                println!("NonePCResultPath NonePCResultPath NonePCResultPath");
                return PrecompiledResult::NonePCResultPath;
            },
        }
        
    }
    return PrecompiledResult::NonePCResultPath;
}

fn exempt_compile_current_source_by_cache(single_source_file: String, compiler_commands: &mut Vec<std::ffi::OsString>) {

    let command = compiler_commands.clone().into_iter().filter(|arg| 
        !arg.to_string_lossy().contains(&single_source_file));
    
    let temp_commands = command.collect::<Vec<std::ffi::OsString>>();
    *compiler_commands = temp_commands;
}

fn determine_whether_need_compile(compiler_commands: Vec<std::ffi::OsString>) -> bool {

    let source = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().contains(".cpp") || 
                        arg.to_string_lossy().contains(".c") || arg.to_string_lossy().contains(".cc")).collect::<Vec<std::ffi::OsString>>();
    if source.len() > 0 {
        return true;
    }
    else {
        return false;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]

    fn test_local_compile() {
        println!("run msvc .cpp file generate obj test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("TEST_DEFINE"));
        compiler_commands.push(std::ffi::OsString::from("/DTEST_DEFINE_NEW"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from(r#""CMAKE_INTDIR=\"Debug\"""#));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);

    }

    #[test]
    fn test_local_compile_warning() {
        println!("run msvc .cpp file generate obj with warning test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("/W4"));
        compiler_commands.push(std::ffi::OsString::from("/DTEST_DEFINE_NEW"));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        compiler_commands.push(std::ffi::OsString::from("/Fowarning.obj"));

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\warning.cpp"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);

    }

    #[test]
    fn test_local_compile_error() {
        println!("run msvc .cpp file generate obj with error test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("/W4"));
        compiler_commands.push(std::ffi::OsString::from("/DTEST_DEFINE_NEW"));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        compiler_commands.push(std::ffi::OsString::from("/Foerror.obj"));

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\error.cpp"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);

    }

    #[test]
    fn test_local_precompile() {
        println!("run msvc .cpp file generate .i test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/P"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/TC"));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        compiler_commands.push(std::ffi::OsString::from("/Filz4.i"));

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        println!("draft dir: {:?}", draft_dir);
        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);
    }

    #[test]
    fn test_local_precompile_by_stdout() {
        println!("run msvc .cpp file generate .i by stdout test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/E")); //stdout
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/TC"));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        //compiler_commands.push(std::ffi::OsString::from("/Filz4.i")); don't need

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        println!("draft dir: {:?}", draft_dir);
        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);
    }

    #[test]
    fn test_local_precompile_by_file() {
        println!("run msvc .cpp file generate .i by disk file test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/P")); //disk file
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/TC"));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        //compiler_commands.push(std::ffi::OsString::from("/Filz4.i")); don't need

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        println!("draft dir: {:?}", draft_dir);
        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);
    }

    #[test]
    fn test_local_compile_preprocessed_file() {
        println!("run msvc compile .i file test");
        tools::logger::init_once_logger();
        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));
        compiler_commands.push(std::ffi::OsString::from("/TC"));

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        println!("draft dir: {:?}", draft_dir);
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.i"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);
    }

    #[test]
    fn test_local_compile_preprocess_file_use_project_arg() {
        println!("run msvc compile use project args test, if you want simulate the project compile, please use this test and replace args.");
        tools::logger::init_once_logger();
        let compiler_commands: Vec<std::ffi::OsString> = vec!["/c", "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\tests", "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\tests", 
            "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\tests\\executiontest_autogen\\include_Debug", "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool", "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\3rdparty", 
            "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable", "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\core", 
            "/I", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\common", "/Zi", "/nologo", "/W1", "/WX-", 
            "/diagnostics:column", "/Od", "/Ob0", "/D_UNICODE", "/DUNICODE", "/DWIN32", "/D_WINDOWS", "/DUNICODE", "/D_UNICODE", "/D_USING_V110_SDK71_=1", 
            "/DQT_DISABLE_DEPRECATED_BEFORE=0x050500", "/DQT_USE_FAST_CONCATENATION", "/DQT_USE_FAST_OPERATOR_PLUS", "/DQT_NO_CAST_TO_ASCII", 
            "/DQT_NO_URL_CAST_FROM_STRING", "/DQT_NO_DEBUG_OUTPUT", "/DQT_TESTLIB_LIB", "/DQT_TESTCASE_BUILDDIR=\\E:/TestFuture/GammaRay/GammaRayTool/build_enable\\", 
            "/DQT_CORE_LIB", "/DQT_GUI_LIB", "/DCMAKE_INTDIR=\\Debug\\", "/Gm-", "/EHsc", "/RTC1", "/MDd", "/GS", "/fp:precise", "/Zc:wchar_t", "/Zc:forScope", "/Zc:inline", "/GR", 
            "/Foexecutiontest.dir\\Debug\\\\", "/Fdexecutiontest.dir\\Debug\\vc143.pdb", "/external:W0", "/Gd", "/TP", "/wd4244", "/wd4267", "/errorReport:prompt", "/external:I", 
            "D:/WorkTool/Qt/qt_5.15.2.17/out64/include", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtTest", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtCore", 
            "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/./mkspecs/win32-msvc", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtGui", "/external:I", 
            "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtANGLE", "/TP", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\tests\\executiontest.dir\\Debug\\mocs_compilation_Debug.i", 
            "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\tests\\executiontest.dir\\Debug\\executiontest.i"].iter().map(|item|std::ffi::OsString::from(*item)).collect::<Vec<std::ffi::OsString>>();

        let current_crate_dir = "E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\tests";
        let current_crate_dir = std::path::PathBuf::from(current_crate_dir);

        let env = crate::platform::windows::WindowsCompilerEnv::default();
        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let _ = request_local_compile(&complier_path.into_os_string(), &current_crate_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);
    }

    #[test]
    fn test_parse_version_from_path() {
        let line = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe";
        let version = parse_version_from_path(line);
        println!("verson: {:?}", version);

    }

    #[test]
    fn test_tidyup_commands_for_precompile() {
        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/EHs /MD /GS /guard:cf /Gy /Qpar /fp:precise /Qspectre /Zc:wchar_t /Zc:forScope /Zc:inline /GR /std:c++17 /permissive-"));
        compiler_commands.push(std::ffi::OsString::from("/TP"));
        compiler_commands.push(std::ffi::OsString::from(r"C:\test.c"));
        compiler_commands.push(std::ffi::OsString::from(r"C:\test.CPP"));
        let commands = tidyup_commands_for_precompile(&compiler_commands);
        
        let mut iter = commands.iter().filter(|&item| item.to_string_lossy().contains("test"));
        println!("tidy up commands {:?}", commands);
        assert_eq!(iter.next(), None);
    }

    use std::os::windows::process::CommandExt;
    #[test]
    fn test_process_command_input_arg() {
        let mut commands: Vec<std::ffi::OsString> = Vec::new();
        commands.push(std::ffi::OsString::from("/C"));
        commands.push(std::ffi::OsString::from("echo"));
        commands.push(std::ffi::OsString::from("/nologo"));
        commands.push(std::ffi::OsString::from(r#"CMAKE_INTDIR="Release""#));

        let mut process = std::process::Command::new("cmd");
        for item in commands {
            process.raw_arg(item);
        }
        let output = process.output().expect("failed to execute process");

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        println!("stdout: {}", stdout_str);

        let mut commands: Vec<&str> = Vec::new();
        commands.push("/C");
        commands.push("echo");
        commands.push("/nologo");
        commands.push(r#"CMAKE_INTDIR="Release""#);

        let out = std::process::Command::new("cmd").args(commands).output().expect("failed to execute process");
        let stdout_str = String::from_utf8_lossy(&out.stdout);
        println!("stdout: {}", stdout_str);
    }

    #[tokio::test]
    async fn test_local_precompile_by_stdout_stream() {
        println!("run msvc .cpp file generate .i by stdout stream test");
        tools::logger::init_once_logger();

        let env = crate::platform::windows::WindowsCompilerEnv::default();

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/E")); //stdout
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/Qpar"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("/TC"));
        
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        //compiler_commands.push(std::ffi::OsString::from("/Filz4.i")); don't need

        let current_crate_dir = env!("CARGO_MANIFEST_DIR");
        let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
        current_crate_dir.pop();
        let draft_dir = current_crate_dir.join("draft");

        println!("draft dir: {:?}", draft_dir);
        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, draft_dir.to_string_lossy())));

        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let resut = start_local_compiler_by_stream(&complier_path.into_os_string(), &draft_dir.into_os_string(), &compiler_commands);

        match resut {
            Ok((mut out, mut err, mut child)) => {
                println!("compile success");
                
                while let Some(line) = out.next_line().await.unwrap() {
                    println!("stdout: {}", line);
                }

                while let Some(line) = err.next_line().await.unwrap() {
                    println!("stderr: {}", line);
                }
      
                let status = child.wait().await.unwrap();
                let code = status.code();
                println!("status code: {:?}", code);
            },
            Err(e) => {
                println!("compile failed: {}", e);
            }
        }
    }
}