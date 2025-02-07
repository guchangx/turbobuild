
//#[cfg(target_os = "windows")]
//extern crate regex;

#[derive(Clone)]
pub struct MSVC {
    pub work_env: crate::platform::windows::WindowsCompilerEnv,
    pub runtime: std::sync::Arc<tokio::runtime::Handle>,
    pub sender: std::sync::Arc<std::sync::Mutex<crate::communicate::distributor::Distributor>>,
}

//TODO common param in func should be move in msvc struct
use std::{io::Read, ops::{Add, Index}};
use crate::compiler::model::{CompilerInput, CompilerOutput, ProcessedResults, PrecompiledSource};

//TODO: rename ProcessedResults to CompileResults

impl crate::compiler::interface::Compiler for MSVC {
    fn request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<ProcessedResults>) {

        let (tx, mut rx) = tokio::sync::oneshot::channel();

        let runtime = self.runtime.clone();
        
        let self_ = self.clone();

        let _ = self.runtime.spawn_blocking(move || {
            let output = runtime.block_on(async move {
                let output = self_.request_msvc_compile(compiler_input).await;
                return output;
            });
            let _ = tx.send(output);
        });

        loop {
            match rx.try_recv() {
                Ok(output) => return output,
                Err(err) => {
                    if err ==  tokio::sync::oneshot::error::TryRecvError::Empty {
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                }
            }
        }
    }
}

impl MSVC {
    async fn request_msvc_compile(self, compiler_input: CompilerInput) -> (CompilerOutput, Option<ProcessedResults>) {

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

        let _source_file = fetch_compile_source_file(&compiler_input.build_and_compiler_type,
            compiler_commands, &compiler_input.compiler_working_dir).unwrap();

        let _object = fetch_compile_object_file(&compiler_input.build_and_compiler_type,
                                    &compiler_commands, &compiler_input.compiler_working_dir);
    
        let exists_source_file_in_command = determine_whether_need_compile(compiler_commands.clone());
        
        if exists_source_file_in_command {

            let mut default_output = CompilerOutput::default();
            let mut default_results = Vec::<crate::compiler::model::ProcessedResult>::new();

            //TODO add expression to determine use local build or dist build
            
            if false {
                let (output, results) = request_local_compile(&compiler_input.compiler_path, &compiler_input.compiler_working_dir,
                                                            compiler_commands, compiler_input.build_and_compiler_type,
                                                            false);
                default_output.set(output);
                if let Some(results) =  results {
                    default_results = results;
                }
            }
            else {
                if true {
                    // dist with preprocessed source
                    let now = std::time::Instant::now();
                    let output = self.request_multi_dist_once_compile(&compiler_input.project, &compiler_input.compiler_path, &compiler_input.compiler_working_dir, &compiler_commands.clone()).await;
                    log::trace!("request_multi_dist_sync_once_compile elaspsed time: {:?}", now.elapsed());
                    //let output = request_dist_compile(&working_parameters.network_client, &compiler_path, &msvc_compile_input.compiler_working_dir, &compiler_commands.clone());
                    default_output.set(output);
                }
                else {
                    //dist with source file and include file
                    let output = request_dist_compile_with_source_and_include(&self.work_env, &compiler_input);
                    default_output.set(output);
                }
            }
            
            //TODO cache the result to redis or cloud

            if default_results.is_empty() {
                return (default_output, None);
            }
            else {
                return (default_output, Some(default_results))
            }        
        }
        else {
            let compiled_filename: Vec<std::ffi::OsString> = Vec::new();
            let result = CompilerOutput {
                filename: compiled_filename,
                status: false,
                output: std::ffi::OsString::from("don't need compile anything."),
            };
            return (result, None);
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

    async fn request_multi_dist_once_compile(&self, project: &std::ffi::OsString, compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> CompilerOutput {

        let mut result = CompilerOutput::default();
        let now = std::time::Instant::now();

        //&self.enrich_compiler_commands(&compiler_input);
        let (status, stdout, stderr) = request_local_precompile(compiler_path, compiler_working_dir, &self.merge_sdk_includes_into_commands(&compiler_commands), false);

        let err = String::from_utf8_lossy(&stderr);

        if status {
            let mut files: Vec<String> = err.lines().map(|item| item.to_string()).collect();

            let actions = parse_action_from_commands(&std::ffi::OsString::from("MSBuild"), compiler_commands, compiler_working_dir);
            
            let source_files: Vec<String> = files.clone();

            let sources = actions.compile_source_file;
            if sources.len() >= 1 {
                let keys = sources.into_keys().collect::<Vec<String>>();
                files.sort();

                if keys == files {
                    log::debug!("local preprocessed multiple sources, count: {:?}. elapsed time: {:?}.", files.len(), now.elapsed());
                }
                else {
                    log::warn!("preprocessed multiple sources are not same with commands. elapsed time: {:?}.", now.elapsed());
                }
            }
            else {
                log::warn!("can't parse source file from commands.");
            }

            let mut commands = tidyup_commands_for_precompile(compiler_commands);

            let addr = self.sender.lock().unwrap().schedule();
            
            if actions.precompile_2_stdout {
                let mut content = String::from_utf8_lossy(&stdout);
                //TODO should not convrt to String
    
                let mut index = 0;
                let mut handles = Vec::new();
    
                let addr_ = addr.clone();

                for file in &source_files {
                    
                    let mut precompiled_result_path = std::path::PathBuf::from(file);
                    match actions.precompiled_result_file.clone() {
                        PrecompiledResult::PathWithPCResultName(path) => {
                            precompiled_result_path = std::path::PathBuf::from(path.to_owned().clone());
                        },
                        PrecompiledResult::PathWithoutPCResultName(path) => {
                            let path = std::path::PathBuf::from(path).join(file);
                            precompiled_result_path = path;
                        },
                        _ => {},
                    }
                    commands.push(precompiled_result_path.clone().into_os_string());
                    
                    if let Some(next_file) = source_files.get(index + 1) {
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
                                    path: std::ffi::OsString::from(&precompiled_result_path)
                                };
    
                                log::debug!("sync precompiled source file {:?}. size: {:.2?}M.", precompiled_result_path.file_name().unwrap(), first.as_bytes().len() as f32 / 1024.0 / 1024.0);
                                content = last.to_string().into();
    
                                let addr__ = addr_.clone();
                                let self_ = self.clone();
    
                                let handle = self.runtime.spawn_blocking( move || {
                                    let _ = self_.request_dist_compile_and_wait_result(&addr__,
                                        &msvc_compile_empty_input, &precompiled_suorce);
                                });
                                
                                handles.push(handle);
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
                        
                        let output = self.runtime.block_on(async move {
                            let output = self.request_dist_compile_and_wait_result(&addr__, &compiler_input, &precompiled_suorce).await;
                            return output;
                        });
                        
                        log::debug!("request {:?} dist compile with precompiled source size: {:?}M, response elapsed: {:?}.", precompiled_result_path.file_name().unwrap(), content.as_bytes().len() as f32 / 1024.0 / 1024.0, now.elapsed());
                
                        let mut file = output.filename;
                        result.filename.append(&mut file);
                        result.output = std::ffi::OsString::from(output.output.to_str().unwrap());
                        result.status = output.status;
    
                        break;
                    }
                    index = index.add(1);
                }
    
                self.sender.lock().unwrap().done(addr.as_str());
 
            }
            else {
                //prcompile to file
                if stdout.is_empty() {

                    result = self.request_dist_compile_from_file(&actions.precompiled_result_file, project, compiler_path, compiler_working_dir, &addr, source_files, &commands).await;

                }
            }
        }
        else {

        }
        return result;
    }

    async fn request_dist_compile_from_file(&self, precompiled_result: &PrecompiledResult, project: &std::ffi::OsString, compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, addr: &str, source_files: Vec<String>, compiler_commands: &Vec<std::ffi::OsString>) 
        -> CompilerOutput {
        let now = std::time::Instant::now();

        let  precompiled_files = self.load_and_transmit_precompiled_result(&source_files, &addr, project, &precompiled_result).await;
        log::debug!("dist sync precompiled source files. count: {:?}, elapsed time {:?}", precompiled_files.len(), now.elapsed());
        let mut commands = compiler_commands.clone();
        for file in precompiled_files {
            commands.push(file);
        }

        let input = CompilerInput {
            project: project.to_owned(),
            compiler_path: compiler_path.to_owned(),
            compiler_working_dir: working_dir.to_owned(),
            compiler_commands: commands.to_owned(),
            build_and_compiler_type: std::ffi::OsString::from("MSBuild_Precompile")
        };

        let precompiled_suorce = crate::compiler::model::PrecompiledSource {
            contents: None,
            path: std::ffi::OsString::new(),
        };

        let output = self.request_dist_compile_and_wait_result(&addr, &input, &precompiled_suorce).await;
        log::debug!("request dist compile without precompiled source files elapsed time {:?}", now.elapsed());
        return output;
    }

    async fn request_dist_compile_from_stdout() {

    }

    async fn request_dist_compile_and_wait_result(&self, addr: &str, input: &CompilerInput, precompiled: &PrecompiledSource) -> CompilerOutput {
    
        let cversion = parse_version_from_path(input.compiler_path.as_os_str().to_str().unwrap()).unwrap();
        log::debug!("in commands compiler version: {:?}", cversion);
        if self.sender.lock().unwrap().check(addr, &cversion) {
            let result = request_dist_compile_with_precompiled_source(addr, &input, &precompiled).await;
            if result.status {
            
            }
            else {
                log::trace!("request remote compile and sync back failed: {:?}", result);
                
            }
            return result;
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

            let bulk_handle = self.runtime.spawn(async move {
                let files = transmit_precompiled_source_file(&addr, &project_name, object, &source_files.clone()).await;
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
            commands.push(std::ffi::OsString::from("/I"));
            commands.push(std::ffi::OsString::from(format!("{}", path.to_string_lossy())));
        }
        commands.push(std::ffi::OsString::from("/I"));
        commands.push(std::ffi::OsString::from(format!("{}", self.work_env.msvc_includes_path.to_string_lossy())));
        return commands;
    }
}

async fn transmit_precompiled_source_file(addr: &str, project_name: &std::ffi::OsString, precompiled_result: PrecompiledResult, source_files: &Vec<String>)-> Vec<std::ffi::OsString> {

    let now = std::time::Instant::now();
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Zstd);
    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut cursor);

    let mut precompiled_files_path = std::vec::Vec::<std::ffi::OsString>::new();
    let mut intermediate = std::path::PathBuf::new();

    for file in source_files.to_owned() {

        match precompiled_result.clone() {
            PrecompiledResult::PathWithPCResultName(path) => {
                intermediate = path;
                intermediate.set_extension("i");
            },
            PrecompiledResult::PathWithoutPCResultName(path) => {
                intermediate = path.join(file);
                intermediate.set_extension("i");
            },
            PrecompiledResult::PCResultNameWithoutPath(path) => {

            }
            _ => {},
        }

        log::debug!("zip precompiled file: {:?}", intermediate);

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
    log::debug!("zip precompiled file elapsed time {:?}", now.elapsed());

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

    let intput = CompilerInput {
        project: project_name.into(),
        ..Default::default()
    };

    crate::communicate::distributor::Distributor::compile(&addr, intermediate.as_os_str().into(), &intput, &content).await;
    log::debug!("transmit precompiled source file to remote server elapsed time {:?}", std::time::Instant::now().elapsed());
    return precompiled_files_path;
    
}

fn request_local_precompile(compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
        compiler_commands: &Vec<std::ffi::OsString>, by_stdout: bool) -> (bool, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    
    let mut commands = compiler_commands.to_owned();
    if by_stdout {
        commands.insert(0, std::ffi::OsString::from(r"/E"));
        //remove '/MP'. /E incompatible with multiprocessing
        commands.retain(|item| !item.to_string_lossy().starts_with("/MP"));
    }
    else {
        commands.insert(0, std::ffi::OsString::from(r"/P"));
        if let Some(arg) = compiler_commands.iter().find(|arg| arg.to_string_lossy().starts_with("/Fo")) {
            let path = arg.to_string_lossy().replace("/Fo", "/Fi").replace("obj", "i").replace("\\\\", "\\");
            commands.insert(1, std::ffi::OsString::from(path));
        }
    }

    let (status, stdout, stderr) = start_local_compiler(compiler_path, compiler_working_dir, &commands);
    return (status, stdout, stderr);
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
                            sync_compile_result: bool) -> (CompilerOutput, Option<ProcessedResults>) {
    let now = std::time::Instant::now();
    let (status, stdout, _stderr) = start_local_compiler(&compiler_path, &compiler_working_dir, &compiler_commands);
    let compile_output = String::from_utf8_lossy(&stdout);
    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    let mut compiled_results: ProcessedResults = Vec::new();

    if status {
        let output = compile_output.lines();

        let mut last = output.clone().last();
        let lines:Vec<&str> = output.clone().collect();
        if let Some(index) = lines.iter().rposition(|&arg| arg.ends_with(".i")) {
            let &arg = lines.index(index);
            last = Some(arg);
        }
        log::trace!("last compiled source file: {:?}", last);
        log::debug!("local compile file count: {:?}, elapsed: {:?}.", lines.len(), now.elapsed());

        let (pdb_path, _one_pdb )= fetch_compile_pdb_path(build_and_compiler_type.clone(), compiler_commands.to_owned(), compiler_working_dir);
        let pdb_path = std::rc::Rc::new(pdb_path);
        for line in output {
            let line = line.replace(r#"""#, "");
            if line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".i") {
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

                    let processed_result = crate::compiler::model::ProcessedResult {
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
    };
    
    let result = CompilerOutput {
        filename: compiled_filename,
        status: status,
        output: std::ffi::OsString::from(compile_output.to_string()),
    };

    return (result, Some(compiled_results));
}

fn request_local_compile_by_preprocessed_source(msvc_compile_input: &CompilerInput, _pool: std::sync::Arc<tokio::runtime::Handle>) -> (CompilerOutput, Option<ProcessedResults>) {

    //replace .cpp/.c to .i
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
    let mut dist_msvc_include_path: std::ffi::OsString = std::ffi::OsString::from("");
    let mut win_kits_include_dir: std::ffi::OsString = std::ffi::OsString::from("");

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

async fn request_dist_compile_with_precompiled_source(addr: &str, input: &CompilerInput, precompiled: &PrecompiledSource) -> CompilerOutput {

    let now = std::time::Instant::now();
    let path = precompiled.path.clone();
    if !input.compiler_commands.is_empty() || !precompiled.contents.is_some() {
        if let Some(content) = precompiled.contents.clone() {
            log::info!("precompiled sourcefile result has content. so just transmit file");
            let content = std::borrow::Cow::from(content);
            crate::communicate::distributor::Distributor::compile(addr, path, input, &content).await;
        }
        else {
            log::info!("precompiled sourcefile result content is empty. so just transmit command"); 
            crate::communicate::distributor::Distributor::compile(addr, path, input, &std::borrow::Cow::from(Vec::new())).await;
        }   
    }
    else {
        log::warn!("compiler commands and context is all empty, so do nothing.")
    }

    log::debug!("communicate distribute compile elapsed: {:?}", now.elapsed());

    return CompilerOutput::default();
}

fn start_local_compiler(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> (bool, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    use std::process::Stdio;

    log::trace!("local compile working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compiler commands: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    let child = std::process::Command::new(compiler_path)
                            .current_dir(working_dir)
                            .args(compiler_commands.clone())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn();
    
    match child {
        Ok(child) => {
            let child_id = child.id();
            match child.wait_with_output() {
                Ok(output) => {
                    if output.status.success() {
                        let elapsed = start.elapsed();
                        let output_context = String::from_utf8_lossy(&output.stdout);  //filename
                        let stderr_context = String::from_utf8_lossy(&output.stderr); 
        

                        log::trace!("compile local file count {:?} success, elapsed time: {:?}, stdout: {:?}, stderr: {:?}", stderr_context.lines().count(), elapsed, output_context, stderr_context);
                        return (true, std::sync::Arc::new(output.stdout), std::sync::Arc::new(output.stderr));
                        //TODO shoud not be used Arc wrap
                    }
                    else {
                        let mut output_context = String::from_utf8_lossy(&output.stdout);
                        if output.stdout.is_empty() {
                            output_context = String::from_utf8_lossy(&output.stderr);
                        }
                        let elapsed = start.elapsed();
                        log::info!("compile file elapsed time: {:?}. cl error message: {:?}, error code: {:?}. output messaage {:?}", elapsed, output_context, output.status.code(), output_context);
                        return (false, std::sync::Arc::new(output.stdout), std::sync::Arc::new(output.stderr));
                    }
                },
                Err(error) => {
                    log::warn!("compile child wait output error: {:?}", error);
                    let mut error_description = String::from("compile child wait output error: ");
                    error_description.push_str(error.to_string().as_str());
                    return (false, std::sync::Arc::new(error_description.into_bytes()), std::sync::Arc::new(vec![]));
                },
            }
        },
        Err(error) => {
            println!("spawn compile child process error: {:?}", error);
            let mut error_description = String::from("spawn compile child process error: ");
            error_description.push_str(error.to_string().as_str());
            return (false, std::sync::Arc::new(error_description.into_bytes()), std::sync::Arc::new(vec![]));
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
                path_contain_space.contains(".cpp") || path_contain_space.contains(".c") {
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
            
            if command.to_lowercase().contains(".cpp") || command.to_lowercase().contains(".c") {
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
            else if command.to_lowercase().contains(".cpp") || command.to_lowercase().contains(".c") {
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
        if item == "/TP" || item == "/TC" || item == "/Tc" || item == "/Tp" {
            specify_sourcefile_type = false;
        }

        cxx = item.ends_with(".cpp") || item.ends_with(".cxx");
        c = item.ends_with(".c");

        return !((cxx || c || item.starts_with("/p")) || item.starts_with("/fi")) 
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

#[derive(Clone)]
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
                        arg.to_string_lossy().contains(".c")).collect::<Vec<std::ffi::OsString>>();
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
    fn test_local_compile_preprocess_file() {
        println!("run msvc compile .i file test");
        tools::logger::init_once_logger();
        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/c"));
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
    fn test_parse_version_from_path() {
        let line = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe";
        let version = parse_version_from_path(line);
        println!("verson: {:?}", version);

    }

    #[test]
    fn test_tidyup_commands_for_precompile() {
        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/EHs /MD /GS /guard:cf /Gy /Qpar /fp:precise /Qspectre /Zc:wchar_t /Zc:forScope /Zc:inline /GR"));
        compiler_commands.push(std::ffi::OsString::from(r"C:\test.c"));
        compiler_commands.push(std::ffi::OsString::from(r"C:\test.CPP"));
        let commands = tidyup_commands_for_precompile(&compiler_commands);
        
        let mut iter = commands.iter().filter(|&item| item.to_string_lossy().contains("test"));
        println!("tidy up commands {:?}", commands);
        assert_eq!(iter.next(), None);
    }
}