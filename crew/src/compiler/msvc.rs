
//#[cfg(target_os = "windows")]
//extern crate regex;


#[derive(Clone)]
pub struct MSVC {
    pub work_env: crate::platform::windows::WindowsCompilerEnv,
    pub runtime: std::sync::Arc<tokio::runtime::Handle>,
    pub sender: std::sync::Arc<std::sync::Mutex<crate::communicate::distributor::Distributor>>,
    pub output_callback: crate::compiler::model::OutputCallback,
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

use std::{io::Read, ops::Index};
use crate::compiler::model::{CompilerInput, CompilerOutput, CompiledResults, PrecompiledSource};
use std::io::BufRead;

use windows_sys::Win32 as win;

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
                if false {
                    // dist with preprocessed source
                    let now = std::time::Instant::now();
                    let output = self.request_multi_dist_batch_compile(compiler_input.clone()).await;
                    log::trace!("request_multi_dist_sync_once_compile: {:?} elaspsed time: {:?}", compiler_input.project, now.elapsed());
                    //let output = request_dist_compile(&working_parameters.network_client, &compiler_path, &msvc_compile_input.compiler_working_dir, &compiler_commands.clone());
                    compiler_output.set(output);
                }
                else {
                    //dist with source file and include file
                    let output = self.request_dist_compile_with_source_and_include(&compiler_input).await;
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
    
        if compiler_input.build_and_compiler_type.to_string_lossy().contains("msbuild") {
            let env =  &self.work_env;
    
            for include in &env.winkits_includes_path {
                if commands.iter().find(|&item| item.to_string_lossy().contains(&include.to_string_lossy().to_string())).is_none() {
                    commands.push(std::ffi::OsString::from("/I"));
                    commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
                }
            }

            for include in &env.msvc_includes_path {
                if commands.iter().find(|&item| item.to_string_lossy().contains(&include.to_string_lossy().to_string())).is_none() {
                    commands.push(std::ffi::OsString::from("/I"));
                    commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
                }
            }
    
            return commands;
        }
        else if  compiler_input.build_and_compiler_type.to_string_lossy().contains("cmake") {
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

        let mut precomiler_commands= compiler_input.compiler_commands.clone();
        if compiler_input.build_and_compiler_type.to_string_lossy().contains("clang_cl") {
            for i in 0 .. precomiler_commands.len() {
                if precomiler_commands[i].to_string_lossy().to_lowercase() == "/c" {
                    precomiler_commands.remove(i);
                    break;
                }
            }

            for i in 0 .. precomiler_commands.len() {
                if precomiler_commands[i].to_string_lossy() == "/showIncludes" {
                    precomiler_commands.remove(i);
                    break;
                }
            }
        }
        else {
            precomiler_commands = self.merge_sdk_includes_into_commands(&compiler_commands);
        }
        
        let outtype = request_local_precompile(&compiler_input.compiler_path, working_dir, &precomiler_commands, false);

        match outtype {
            Ok(out) => {
                match out {
                    crate::compiler::msvc::OutType::Std(_stdout) => {
                        //handle_compile_by_std(stdout);
                    },
                    crate::compiler::msvc::OutType::File(mut fileout) => {

                        let addr = self.sender.lock().unwrap().schedule();
                        output = self.handle_compile_by_filestream(&mut fileout, compiler_input.clone(), &addr, &compiler_commands).await;

                        match fileout.child.wait() {
                            Ok(exit) => {
                                let code = exit.code().unwrap_or(-1);
                                log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input.project, code, now.elapsed());

                            },
                            Err(err) => {
                                log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input.project, err, now.elapsed());
                            }
                        }
                    },
                }
            },
            Err(err) => {
                log::warn!("precompile {:?} failed: {:?}", compiler_input.project, err);
                output.status = 1;
            },
        }

        log::debug!("local {:?} precompile and dist file and commmand done. elaspsed time: {:?}", compiler_input.project, now.elapsed());
        return output;
    }


    async fn request_multi_dist_batch_compile(&self, compiler_input: CompilerInput) -> CompilerOutput {

        let mut output = CompilerOutput::default();
        let now = std::time::Instant::now();

        let working_dir = compiler_input.compiler_working_dir.clone();
        let compiler_commands = compiler_input.compiler_commands.clone();
        let compiler_path = compiler_input.compiler_path.clone();

        let mut precomiler_commands= compiler_commands.clone();
        if compiler_input.build_and_compiler_type.to_string_lossy().contains("clang_cl") {
            for i in 0 .. precomiler_commands.len() {
                if precomiler_commands[i].to_string_lossy().to_lowercase() == "/c" {
                    precomiler_commands.remove(i);
                    break;
                }
            }

            for i in 0 .. precomiler_commands.len() {
                if precomiler_commands[i].to_string_lossy() == "/showIncludes" {
                    precomiler_commands.remove(i);
                    break;
                }
            }
            
            precomiler_commands = self.merge_sdk_includes_into_commands(&compiler_commands.clone());

            let outtype = request_local_precompile(&compiler_path, &working_dir, &precomiler_commands, false);

            match outtype {
                Ok(out) => {
                    match out {
                        crate::compiler::msvc::OutType::Std(_stdout) => {
                            //handle_compile_by_std(stdout);
                        },
                        crate::compiler::msvc::OutType::File(mut fileout) => {

                            let addr = self.sender.lock().unwrap().schedule();
                            output = self.handle_compile_by_filestream(&mut fileout, compiler_input.clone(), &addr, &compiler_commands.clone()).await;

                            match fileout.child.wait() {
                                Ok(exit) => {
                                    let code = exit.code().unwrap_or(-1);
                                    log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input.project, code, now.elapsed());

                                },
                                Err(err) => {
                                    log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input.project, err, now.elapsed());
                                }
                            }
                        },
                    }
                },
                Err(err) => {
                    log::warn!("precompile {:?} failed: {:?}", compiler_input.project, err);
                    output.status = 1;
                },
            }

            log::debug!("local {:?} precompile and dist file and commmand done. elaspsed time: {:?}", compiler_input.project, now.elapsed());
            return output;
        }
        else {

            let mut handles = Vec::new();
            let mut pdb = std::ffi::OsString::from("");

            let mut compiler_commands_ = compiler_commands.clone();
            compiler_commands_.retain(|item| {
                let cmd = item.to_string_lossy().to_lowercase();
                if cmd.starts_with("/fd") {
                    pdb = item.clone();
                    return false;
                }
                else {
                    return true;
                }
            });

            let mut sources = Vec::new();
            let mut commands = Vec::new();

            (sources, commands) = compiler_commands_.iter().partition(|item| {
                let cmd = item.to_string_lossy().to_lowercase();

                if cmd.ends_with(".cpp") || cmd.ends_with(".c") || cmd.ends_with(".cxx") || cmd.ends_with(".cc") {
                    return true;
                }
                else {
                    return false;
                }
            });

            let batch_size: usize = 40;
            let len = sources.len();

            log::debug!("project: [{:?}] sources len: {:?} batch size: {:?}", &compiler_input.project, len, &batch_size);

            let mut index = 0;
            if len > batch_size {
                let mut batch = Vec::new();
                for (i, source) in sources.iter().enumerate() {
                    batch.push(source);
                    
                    if (i + 1) % batch_size == 0 || i == len - 1 {
                        let mut compiler_commands = Vec::new();
                        let mut last = false;
                        if len <= batch_size / 3 + i {
                            batch.append(&mut sources[i + 1..].into_iter().collect());
                            compiler_commands = self.merge_sdk_includes_into_commands(&commands.iter().map(|&item| item.clone()).collect());
                            compiler_commands.append(&mut batch.iter().map(|&&item| item.clone()).collect());

                            last = true;
                        } 
                        else {
                            compiler_commands = self.merge_sdk_includes_into_commands(&commands.iter().map(|&item| item.clone()).collect());
                            compiler_commands.append(&mut batch.iter().map(|&&item| item.clone()).collect());
                        }

                        if index == 0 {
                            if !pdb.is_empty() {
                                compiler_commands.push(pdb.clone());
                            }
                        }
                        else {
                            let pdb = pdb.to_string_lossy().to_string();
                            if pdb.ends_with(".pdb") {
                                let path = format!("{}_tb_{}.pdb", &pdb[..pdb.len()-4], index);
                                compiler_commands.push(path.into());
                            }
                            else {
                                let path = format!("{}/{}_tb_{}.pdb", pdb, "vc143", index);
                                compiler_commands.push(path.into());
                            }
                        }

                        let self_ = self.clone();

                        let working_dir = working_dir.clone();
                        let compiler_path_ = compiler_path.clone();

                        let compiler_input_ =  CompilerInput {
                            solution: compiler_input.solution.clone(), 
                            project: compiler_input.project.clone(), 
                            compiler_path: compiler_path.clone(),
                            compiler_working_dir: working_dir.clone(),
                            compiler_commands: compiler_commands.clone(),
                            build_and_compiler_type: compiler_input.build_and_compiler_type.clone(),
                            envs: std::collections::HashMap::new()
                        };

                        let handle = self.runtime.spawn(async move {

                            let mut output = CompilerOutput::default();

                            let outtype = request_local_precompile(&compiler_path_, &working_dir, &compiler_commands.clone(), false);

                            match outtype {
                                Ok(out) => {
                                    match out {
                                        crate::compiler::msvc::OutType::Std(_stdout) => {
                                            //handle_compile_by_std(stdout);
                                        },
                                        crate::compiler::msvc::OutType::File(mut fileout) => {

                                            let addr = self_.sender.lock().unwrap().schedule();
                                            
                                            output = self_.handle_compile_by_filestream(&mut fileout, compiler_input_.clone(), &addr, &compiler_commands.clone()).await;

                                            match fileout.child.wait() {
                                                Ok(exit) => {
                                                    let code = exit.code().unwrap_or(-1);
                                                    log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input_.project, code, now.elapsed());

                                                },
                                                Err(err) => {
                                                    log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input_.project, err, now.elapsed());
                                                }
                                            }
                                        },
                                    }
                                },
                                Err(err) => {
                                    log::warn!("precompile {:?} failed: {:?}", compiler_input_.project, err);
                                    output.status = 1;
                                },
                            }
                            return output;
                        });
                        handles.push(handle);

                        index += 1;
                        batch.clear();

                        if last {
                            break;
                        }
                    }
                }

                //wait for all tasks to complete

                let mut output = CompilerOutput::default();
                let mut out: Vec<u8> = Vec::new();
                let mut err: Vec<u8> = Vec::new();

                for handle in handles {
                    let result = handle.await;
                    match result {
                        Ok(res) => {
                            output.status = res.status;
                            out.extend(res.out.as_ref());
                            err.extend(res.err.as_ref());
                        },
                        Err(err) => {
                            output.status = 1;
                            log::error!("request dist compile failed: {:?}", err);
                        }
                    }
                }
                output.out = std::sync::Arc::new(out);
                output.err = std::sync::Arc::new(err);
                log::trace!("request_multi_dist_batch_compile: {:?} done.", compiler_input.project);
                return output;
            }
            else {
                precomiler_commands = self.merge_sdk_includes_into_commands(&compiler_commands.clone());

                let outtype = request_local_precompile(&compiler_path, &working_dir, &precomiler_commands, false);

                match outtype {
                    Ok(out) => {
                        match out {
                            crate::compiler::msvc::OutType::Std(_stdout) => {
                                //handle_compile_by_std(stdout);
                            },
                            crate::compiler::msvc::OutType::File(mut fileout) => {

                                let addr = self.sender.lock().unwrap().schedule();
                                output = self.handle_compile_by_filestream(&mut fileout, compiler_input.clone(), &addr, &compiler_commands.clone()).await;

                                match fileout.child.wait() {
                                    Ok(exit) => {
                                        let code = exit.code().unwrap_or(-1);
                                        log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input.project, code, now.elapsed());

                                    },
                                    Err(err) => {
                                        log::info!("precompile {:?} status code: {:?}, elapsed time: {:?}", compiler_input.project, err, now.elapsed());
                                    }
                                }
                            },
                        }
                    },
                    Err(err) => {
                        log::warn!("precompile {:?} failed: {:?}", compiler_input.project, err);
                        output.status = 1;
                    },
                }

                log::debug!("local {:?} precompile and dist file and commmand done. elaspsed time: {:?}", compiler_input.project, now.elapsed());
                return output;
            }
        }
    }
    
    async fn request_dist_compile_with_command(&self, addr: &str, compiler_input: CompilerInput, requires: &crate::compiler::model::PrecompiledSource)
        -> CompilerOutput {

        let output = self.request_dist_compile(&addr, &compiler_input, &requires).await;
        return output;
    }

    async fn request_dist_compile(&self, addr: &str, input: &CompilerInput, precompiled: &PrecompiledSource) -> CompilerOutput {
    
        let cversion = parse_version_from_path(input.compiler_path.as_os_str().to_str().unwrap()).unwrap();
        log::debug!("{:?} in commands compiler version: {:?}, addr: {:?}", input.project, cversion, addr);
        if input.build_and_compiler_type.to_string_lossy().contains("clang_cl") {
            let output = request_dist_compile_with_precompiled_source(addr, &input, &precompiled, &self.runtime, self.output_callback.clone()).await;
            if output.status == 0 {
            
            }
            else {
                //log::trace!("request remote compile and sync back failed: {:?} {:?}", output.out, output.err);
            }
            return output;
        }
        else {
            if self.sender.lock().unwrap().check(addr, &cversion) {
            
                let output = request_dist_compile_with_precompiled_source(addr, &input, &precompiled, &self.runtime, self.output_callback.clone()).await;
                if output.status == 0 {
                
                }
                else {
                    //log::trace!("request remote compile and sync back failed: {:?} {:?}", output.out, output.err);
                }
                return output;
            }
            else {
                log::error!("dist compile failed. addr: {} no available remote compiler {:?}", addr, cversion);
    
                return CompilerOutput::default();
            }
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

        for path in &self.work_env.msvc_includes_path {
            if !commands.contains(&path) {
                commands.push(std::ffi::OsString::from("/I"));
                commands.push(std::ffi::OsString::from(format!("{}", path.to_string_lossy())));
            }
        }

        return commands;
    }
    
    async fn handle_compile_by_filestream(&self, fileout: &mut FileOut, compiler_input: CompilerInput, addr: &str, compiler_commands: &Vec<std::ffi::OsString>) 
        -> CompilerOutput {

        let actions = parse_action_from_commands(&std::ffi::OsString::from("msbuild"), compiler_commands, &compiler_input.compiler_working_dir);

        while let Some(line) = fileout.out.next() {
            log::debug!("precompile stdout: {:?}", line);
        }

        let mut handles = Vec::new();
        let (stream, notify) = crate::communicate::distributor::Distributor::archive_stream(&addr, &self.runtime).await;

        let mut errors = Vec::new();

        while let Some(line) = fileout.err.next() {
            log::debug!("precompile stderr: {:?}", line);

            let file = line.unwrap();
            if file.contains("Generating Code...") || file.contains("Compiling...")
            {
    
            }
            else if file.starts_with("Microsoft (R) ") || file.starts_with("Copyright (C) ") {

            }
            else if file.contains(" error: ") || file.contains(": fatal error ") {
                errors.push(file);
            }
            //including file
            else if file.starts_with("Note: ") {

            }
            else if !file.is_empty() {
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

            if compiler_input.build_and_compiler_type.to_string_lossy().contains("clang_cl") {

                let mut files = Vec::new();
                match &actions.precompiled_result_file {
                    crate::compiler::msvc::PrecompiledResult::PathWithPCResultName(file) => {
                        files = transmit_precompiled_source_file(&compiler_input, stream, actions.precompiled_result_file.clone(), &vec![file.to_string_lossy().to_string()], &self.runtime).await;
                    },
                    _ => {
                        log::warn!("no precompiled result file found in command parse. {:?}", &actions.precompiled_result_file);
                    },
                };

                let mut compiler_input = compiler_input.to_owned();
                let mut commands = tidyup_commands_for_precompile(compiler_commands);
                commands.append(&mut files);
                compiler_input.compiler_commands = commands;

                if !std::path::PathBuf::from(&compiler_input.compiler_path).is_absolute() {
                    compiler_input.compiler_path = std::path::PathBuf::from(&compiler_input.compiler_working_dir).join(&compiler_input.compiler_path).into_os_string();
                }
    
                notify.notified().await;

                let mut requires = crate::compiler::model::PrecompiledSource {
                    contents: None,
                    path: std::ffi::OsString::new(),
                };

                let requires_params = commands_dist_parameters_requires(&compiler_input);
                if !requires_params.is_empty() {
                    let (contents, path) = crate::communicate::packager::Packager::pack_separate_file(&requires_params.to_str().unwrap());
                
                    requires = crate::compiler::model::PrecompiledSource {
                        contents: Some(contents.to_vec()),
                        path: path,
                    };
                }
    
                let output = self.request_dist_compile_with_command(&addr, compiler_input.clone(), &requires).await;
                return output;
            }
            else
            {
                log::warn!("{:?} no precompiled source file found in precompile output.", compiler_input.project);
                drop(stream);

                let mut out = CompilerOutput::default();
                out.err = std::sync::Arc::new(errors.join("\n").into_bytes());
                out.status = 1;
                return out;
            }
        }
        else
        {
            let mut files = Vec::new();
            for handle in handles {
                let mut files_ = handle.await.unwrap();
                files.append(&mut files_);
            }

            drop(stream);
    
            let mut compiler_input = compiler_input.to_owned();
            let mut commands = tidyup_commands_for_precompile(compiler_commands);
            commands.append(&mut files);
            compiler_input.compiler_commands = commands;
            
            let mut requires = crate::compiler::model::PrecompiledSource {
                contents: None,
                path: std::ffi::OsString::new(),
            };
            if compiler_input.build_and_compiler_type.to_string_lossy().contains("msvc") {
                if compiler_input.compiler_path.to_string_lossy().contains("~1") {
                    compiler_input.compiler_path = winapi_get_long_path_name(&compiler_input.compiler_path);
                }
            }
            else if compiler_input.build_and_compiler_type.to_string_lossy().contains("clang_cl") {
                let requires_params = commands_dist_parameters_requires(&compiler_input);
                if !requires_params.is_empty() {
                    let (contents, path) = crate::communicate::packager::Packager::pack_separate_file(&requires_params.to_str().unwrap());
                
                    requires = crate::compiler::model::PrecompiledSource {
                        contents: Some(contents.to_vec()),
                        path: path,
                    };
                }
            }

            notify.notified().await;

            let output = self.request_dist_compile_with_command(&addr, compiler_input.clone(), &requires).await;
            return output;
        }
    }

    async fn request_dist_compile_with_source_and_include(&self, input: &CompilerInput) -> CompilerOutput {
        log::debug!("request dist compile with source and include file.");

        let mut set = tokio::task::JoinSet::new();
        let len = self.sender.lock().unwrap().all().len();

        let actions = parse_action_from_commands(&std::ffi::OsString::from("msbuild"), &input.compiler_commands, &input.compiler_working_dir);

        let others = actions.compile_instruction;

        let mut sources = Vec::new();
        let mut sources_dir = std::collections::HashSet::new();
        for (_, path) in &actions.compile_source_file {
            sources.push(path.clone().into_os_string());
            path.parent().map(|parent| {
                sources_dir.insert(parent.to_owned());
            });
        }

        let mut headers = std::collections::HashSet::new();
        for dir in sources_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map(|ext| ext == "h" || ext == "hpp" 
                                || ext == "hh" || ext == "hxx" || ext == "h++" || ext == "inl").unwrap_or(false) {
                            headers.insert(path);
                        }
                    }
                }
            }
        }
        
        let headers = std::sync::Arc::new(tokio::sync::Mutex::new(headers));

        for _ in 0..len {
            let mut addr = String::new();
            let mut index = -1;
            let mut left = Vec::new();
            (addr, index, left, sources) = self.sender.lock().unwrap().schedule_for_sources(None, &sources);
            // if addr is local, we can skip the dist compile

            if !left.is_empty() {
                let self_ = self.clone();

                let solution = input.solution.to_string_lossy().to_string();
                let project = input.project.to_string_lossy().to_string();

                let input_ = input.clone();

                let mut others_ = others.clone();

                let headers = headers.clone();
                set.spawn(async move {
                    
                    let (stream, notify) = crate::communicate::distributor::Distributor::archive_stream(&addr, &self_.runtime).await;

                    //sync source files and same name header files.
                    if let Some(stream) = stream {

                        let mut archive_stream_task = Vec::new();

                        for file in left.clone() {
                            let solution_ = solution.clone();
                            let project_ = project.clone();

                            let stream_ = stream.clone();
                            let stream__ = stream.clone();
                            let headers = headers.clone();

                            let file_ = file.clone();
                            let stask = self_.runtime.spawn(async move {
                                let path = std::path::PathBuf::from(&file);
                                let content = tokio::fs::read(&path).await.unwrap_or_else(|_| {
                                    log::error!("failed to read file: {:?}", file);
                                    Vec::new()
                                });

                                let name = path.file_name().unwrap().to_string_lossy().to_string();

                                let archive = crate::communicate::package::ArchiveArgs {
                                    file_type:  crate::communicate::package::FileType::SourceFiles,
                                    solution: solution_.clone(),
                                    project: project_.clone(),
                                    name: name.clone(),
                                    //path: intermediate_.join(&name).to_string_lossy().to_string(),
                                    path: path.to_string_lossy().to_string(),
                                    content: content.into(),
                                };

                                let _ = stream_.send(archive).await;
                            });
                            archive_stream_task.push(stask);

                            let solution__ = solution.clone();
                            let project__ = project.clone();
                            let htask = self_.runtime.spawn(async move {
                                //.inc;.rc;.resx;.idl;.rc2;.def
                                //.odl;.asm;.asmx;.xsd;.bin;.rgs;.html;.htm;.manifest
                                //.cpp;.cxx;.cc;.c;.c++;.cppm;.ixx;.inl;.ipp
                                //.h;.hh;.hpp;.hxx;.h++;.hm
                                let path = std::path::PathBuf::from(&file_);
                                let header_file = {
                                    let guard = headers.lock().await;
                                    if let Some(found) = guard.iter().find(|&item| item.file_stem().map(|item| item == path.file_stem().unwrap()).unwrap_or(false)) {
                                        Some(found.to_owned())
                                    }
                                    else {
                                        None
                                    }
                                };
    
                                if let Some(found) = header_file {
                                    { headers.lock().await.remove(&found) };
                                    if found.exists() {
                                        let content = tokio::fs::read(&found).await.unwrap_or_else(|_| {
                                            log::error!("failed to read file: {:?}", found);
                                            Vec::new()
                                        });
    
                                        let name = found.file_name().unwrap().to_string_lossy().to_string();
    
                                        let archive = crate::communicate::package::ArchiveArgs {
                                            file_type: crate::communicate::package::FileType::SourceFiles,
                                            solution: solution__.clone(),
                                            project: project__.clone(),
                                            name: name.clone(),
                                            //path: intermediate_.join(name).to_string_lossy().to_string(),
                                            path: found.to_string_lossy().to_string(),
                                            content: content.into(),
                                        };
    
                                        let _ = stream__.send(archive).await;
                                    }
                                }
                            });
                            archive_stream_task.push(htask);
                        }

                        //cancel initiative sync other dep files and header files.
                        /*
                            //sync other dep files and header files
                            for path in header_files.lock().await.iter() {
                                let content = tokio::fs::read(&path).await.unwrap_or_else(|_| {
                                    log::error!("failed to read deps file: {:?}", path);
                                    Vec::new()
                                });

                                let name = path.file_name().unwrap().to_string_lossy().to_string();

                                let archive = crate::communicate::package::ArchiveArgs {
                                    file_type:  crate::communicate::package::FileType::SourceFiles, //TODO: should add HeaderFiles
                                    solution: solution_.clone(),
                                    project: project_.clone(),
                                    name: name.clone(),
                                    //path: intermediate_.join(name).to_string_lossy().to_string(),
                                    path: path.to_string_lossy().to_string(),
                                    content: content.into(),
                                };

                                let _ = stream.send(archive).await;
                            }                        
                        */

                        for task in archive_stream_task {
                            let _ = task.await; 
                        }
                    }

                    notify.notified().await;
                    
                    let requires = crate::compiler::model::PrecompiledSource {
                        contents: None,
                        path: std::ffi::OsString::new(),
                    };

                    if index != 0 {
                        for item in others_.iter_mut() {
                            if item.to_string_lossy().starts_with("/Fd") {
                                if item.to_string_lossy().ends_with(".pdb") {
                                    *item = std::ffi::OsString::from(format!("{}_tb_{}.pdb", item.to_string_lossy().strip_suffix(".pdb").unwrap(), index));
                                }
                                else {
                                    *item = std::ffi::OsString::from(format!("{}/v143_tb_{}.pdb", item.to_string_lossy(), index));
                                }
                                log::debug!("modify pdb path for tb  {:?}", item);
                            }
                        }
                    }

                    let mut input = input_.clone();
                    input.compiler_commands = others_;
                    input.compiler_commands.append(&mut left.iter().map(|item| item.clone()).collect());
                    
                    let output = self_.request_dist_compile(&addr, &input, &requires).await;
                    return (addr, output);
                });
            }
            else  {
                log::error!("no available addr to schedule for dist compile.");
            }

            if sources.is_empty() {
                break;
            }
        }

        while let Some(handle) = set.join_next().await {
            match handle {
                Ok((addr, output)) => {
                    log::debug!("dist compile with source and include file output: {:?}", addr);

                    if !sources.is_empty() {
                        let mut index = -1;
                        let addr_ = addr.clone();
                        let mut left = Vec::new();
                        (_, index, left, sources) = self.sender.lock().unwrap().schedule_for_sources(Some(addr), &sources);

                        let self_ = self.clone();
                        let input_ = input.clone();
                        let mut others_ = others.clone();
                        set.spawn(async move {
                            let (stream, notify) = crate::communicate::distributor::Distributor::archive_stream(&addr_, &self_.runtime).await;
                            
                            if let Some(stream) = stream {
                                let mut archive_stream_task = Vec::new();

                                for file in left.clone() {
                                    let solution = input_.solution.clone();
                                    let project = input_.project.clone();
                                    let stream = stream.clone();

                                    let task = self_.runtime.spawn(async move {
                                        let content = tokio::fs::read(std::path::PathBuf::from(&file)).await.unwrap_or_else(|_| {
                                            log::error!("failed to read file: {:?}", file);
                                            Vec::new()
                                        });
    
                                        let archive = crate::communicate::package::ArchiveArgs {
                                            file_type: crate::communicate::package::FileType::SourceFiles,
                                            solution: solution.to_string_lossy().to_string(),
                                            project: project.to_string_lossy().to_string(),
                                            name: file.to_string_lossy().to_string(),
                                            path: file.to_string_lossy().to_string(),
                                            content: content.into(),
                                        };
    
                                        let _ = stream.send(archive).await;
                                    });

                                    archive_stream_task.push(task);
                                }
                                for task in archive_stream_task {
                                    let _ = task.await; 
                                }
                            }

                            notify.notified().await;

                            let requires = crate::compiler::model::PrecompiledSource {
                                contents: None,
                                path: std::ffi::OsString::new(),
                            };

                            if index != 0 {
                                for item in others_.iter_mut() {
                                    if item.to_string_lossy().starts_with("/Fd") {
                                        if item.to_string_lossy().ends_with(".pdb") {
                                            *item = std::ffi::OsString::from(format!("{}_tb_{}.pdb", item.to_string_lossy().strip_suffix(".pdb").unwrap(), index));
                                        }
                                        else {
                                            *item = std::ffi::OsString::from(format!("{}/v143_tb_{}.pdb", item.to_string_lossy(), index));
                                        }
                                        log::debug!("modify pdb path for tb  {:?}", item);
                                    }
                                }
                            }

                            let mut input = input_.clone();
                            input.compiler_commands = others_;
                            input.compiler_commands.append(&mut left.iter().map(|item| item.clone()).collect());

                            let output = self_.request_dist_compile(&addr_, &input, &requires).await;
                            
                            return (addr_, output);
                        });
                    }
                },
                Err(error) => {
                    log::error!("dist compile with source and include file error: {:?}", error);
                }
            }
        }

        return CompilerOutput::default();
    }

}

fn winapi_get_long_path_name(path: &std::ffi::OsString) -> std::ffi::OsString {
    use std::os::windows::ffi::OsStrExt;
    unsafe {
        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        
        let mut buffer: Vec<u16> = vec![0; win::Foundation::MAX_PATH as usize];
        let len = win::Storage::FileSystem::GetLongPathNameW(
            wide_path.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as u32
        );
        
        if len > buffer.len() as u32 || len == 0 {
            return path.clone();
        }
        buffer.truncate(len as usize);
        return std::ffi::OsString::from(String::from_utf16_lossy(&buffer));
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
                build_and_compiler_type: std::ffi::OsString::from("msbuild precompile")
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
        match intermediate.extension() {
            Some(ext) => {
                if ext == "dir" {
                    let mut dir = intermediate.into_os_string();
                    dir.push(".zip");
                    intermediate = std::path::PathBuf::from(dir);
                }
            },
            None => {
                intermediate.set_extension("zip");
            }
        }
    }   
    else {
        match intermediate.extension() {
            Some(ext) => {
                if ext == "dir" {
                    let mut dir = intermediate.into_os_string();
                    dir.push(".zip");
                    intermediate = std::path::PathBuf::from(dir);
                }
            },
            None => {
                intermediate.set_extension("zip");
            }
        }
    }

    let file = crate::communicate::package::ArchiveArgs {
        file_type:  crate::communicate::package::FileType::PrecompiledSrcFiles,
        solution: compiler_input.solution.to_string_lossy().to_string(),
        project: compiler_input.project.to_string_lossy().to_string(),
        name: source_files.join(",").into(),
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

    let stem = path.file_stem().unwrap();
    if stem == "cl" {
        // C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
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
    }
    else if stem == "clang-cl" {
        // \\third_party\\llvm-build\\Release+Asserts\\bin\\clang-cl.exe
        let cversion = crate::replica::toolchain::CompilerVersion {
            version: "clang-cl.exe".to_string(),
            host: crate::replica::toolchain::Arch::x64,
            target: crate::replica::toolchain::Arch::unknown,
        };
        return Some(cversion);
    }
    else {
        return None;
    }

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
            if line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".cc") || line.ends_with(".cxx") || line.ends_with(".i") {
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

async fn request_dist_compile_with_precompiled_source(addr: &str, input: &CompilerInput, precompiled: 
    &PrecompiledSource, runtime: &std::sync::Arc<tokio::runtime::Handle>, output_callback: crate::compiler::model::OutputCallback) 
    -> CompilerOutput {

    let mut output = CompilerOutput::default();

    let now = std::time::Instant::now();
    let path = precompiled.path.clone();
    if !input.compiler_commands.is_empty() || !precompiled.contents.is_some() {
        if let Some(content) = precompiled.contents.clone() {
            log::info!("precompiled sourcefile result has content. so just transmit file");
            let content = std::borrow::Cow::from(content);
            let receiver = crate::communicate::distributor::Distributor::compile(addr, path, input, &content, runtime, output_callback).await;
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
            let receiver = crate::communicate::distributor::Distributor::compile(addr, path, input, &std::borrow::Cow::from(Vec::new()), runtime, output_callback).await;
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

    log::debug!("communicate distribute compile: {:?} elapsed:{:?}", input.project, now.elapsed());

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
                        
                        log::trace!("compile local file count {:?} success, elapsed time: {:?}, file: {:?}, warning: {:?}, stderr: {:?}, stdout: {:?}", files.len(), elapsed, files, warning_or_message, stderr_context, output_context);
                
                        return (output.status.code().expect("process exit code unwrap failed.") as u32, std::sync::Arc::new(output.stdout), std::sync::Arc::new(output.stderr));
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
    
    let mut compiler = compiler_path.to_owned();

    if !std::path::PathBuf::from(compiler_path).is_absolute() {
        compiler = std::path::PathBuf::from(working_dir).join(compiler_path).into_os_string();
    }

    log::trace!("working: {:?}", working_dir);
    log::trace!("compiler: {:?}", compiler);
    log::trace!("commands: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    let mut process = std::process::Command::new(compiler);

    for item in compiler_commands {
        if item.to_string_lossy().contains(" ") {
            process.arg(item);
        }
        else if item.to_string_lossy().starts_with("-D") && item.to_string_lossy().contains(r#"=""#) {
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
                path_contain_space.contains(".cpp") || path_contain_space.contains(".c") || path_contain_space.contains(".cc") || path_contain_space.contains(".cxx") {
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
    if compiler_input.build_and_compiler_type.to_string_lossy().contains("msbuild") {
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

        for include in working_compiler_env.msvc_includes_path.iter() {
            let mut instruct = "/I".to_string();
            instruct += include.to_str().unwrap();
            args.push(std::ffi::OsString::from(instruct));
        }
        
        return (args, compiler_path);
    }
    else if  compiler_input.build_and_compiler_type.to_string_lossy().contains("cmake") {
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
    if build_and_compiler_type.to_string_lossy().contains("msbuild")
        || build_and_compiler_type.to_string_lossy().contains("cmake")
        || build_and_compiler_type.to_string_lossy().contains("dist") {
        let mut sourcefile: std::collections::HashMap<String, std::path::PathBuf> = std::collections::HashMap::new();
        
        for command in compiler_commands {
            let command = command.to_string_lossy();
            
            if command.to_lowercase().contains(".cpp") || command.to_lowercase().contains(".c") || command.to_lowercase().contains(".cc") || command.to_lowercase().contains(".cxx") {
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
        if item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c") || item.ends_with(".cc") || item.ends_with(".cxx") {
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


    let (files, warning):(Vec<_>, Vec<_>) = lines.into_iter().partition(|item| item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c") || item.ends_with(".cc") || item.ends_with(".cxx"));
    return (files, warning);
}

#[derive(Debug)]
struct CompileAction {
    pub precompile_2_stdout: bool,
    pub compile_source_file: std::collections::HashMap<std::string::String, std::path::PathBuf>,
    pub compile_instruction: Vec<std::ffi::OsString>,
    pub precompiled_result_file: PrecompiledResult,
    pub object_file: GeneratedObject,
    pub pdb_file: ProgramDataBase,
}

fn parse_action_from_commands(build_and_compiler_type: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) -> CompileAction {

    let working_dir = std::path::PathBuf::from(working_dir);

    let mut precompile_2_stdout = false;
    let mut sourcefile: std::collections::HashMap<String, std::path::PathBuf> = std::collections::HashMap::new();
    let mut precompiled_result_file = PrecompiledResult::NonePCResultPath;
    let mut object_file = GeneratedObject::NoneObjPath;
    let mut pdb_file = ProgramDataBase::NonePDBPath;

    let mut compile_other_commands: Vec<std::ffi::OsString> = Vec::new();

    if build_and_compiler_type.to_string_lossy().contains("msbuild")
        || build_and_compiler_type.to_string_lossy().contains("cmake")
        || build_and_compiler_type.to_string_lossy().contains("dist") {

        for command in compiler_commands {
            let mut isfile = false;
            let mut command = command.to_string_lossy();
            if command == "/P" {
                precompile_2_stdout = false;
            }
            else if command == "/E" {
                precompile_2_stdout = true;
            }
            else if command.to_lowercase().ends_with(".cpp") || command.to_lowercase().ends_with(".c") || command.to_lowercase().ends_with(".cc") 
                || command.to_lowercase().ends_with(".cxx") {

                isfile = true;
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
                (pdb_file, _) = exact_compile_pdb_file(command.clone(), &working_dir);
            }
            else if command.starts_with("/Fo") {
                object_file = exact_compile_object_file(command.clone(), &working_dir);
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

            if !isfile {
                compile_other_commands.push(std::ffi::OsString::from(command.into_owned()));
            }
        }

        match precompiled_result_file {
            PrecompiledResult::NonePCResultPath => {
                match object_file {
                    GeneratedObject::PathWithObjName(ref mut path) => {
                        path.set_extension("i");
                        precompiled_result_file = PrecompiledResult::PathWithPCResultName(path.clone());
                    },
                    GeneratedObject::PathWithoutObjName(ref path) => {
                        precompiled_result_file = PrecompiledResult::PathWithoutPCResultName(path.clone());
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
        compile_instruction: compile_other_commands,
        precompiled_result_file: precompiled_result_file,
        object_file: object_file,
        pdb_file: pdb_file,
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

        let removed = item == "/p" || item.starts_with("/fi") || item == "/P" || item.starts_with("/Fi") 
            || item.starts_with("/showincludes") || item.starts_with("/showIncludes") 
            || item.starts_with("-imsvc");

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

fn commands_dist_parameters_requires(input: &CompilerInput) -> std::ffi::OsString {

    let txt =  input.compiler_commands.iter().find(|arg| {
        let arg = arg.to_string_lossy();
        if arg.starts_with("--warning-suppression-mappings=") {
            return true;
        }
        false
    });

    let mut extra_requires = std::ffi::OsString::new();

    if let Some(txt) = txt {
        let txt = txt.to_string_lossy().to_owned();
        let (_, txt) = txt.split_at("--warning-suppression-mappings=".len());
        if !txt.is_empty() {
            let mut path = std::path::PathBuf::from(&txt);
            if !path.has_root() {
                path = std::path::PathBuf::from(&input.compiler_working_dir).join(path);
                let path = std::ffi::OsString::from(path.to_str().unwrap());
                extra_requires = path;
            }
            if path.exists() {
                log::debug!("pickup extra requires for precompile: {:?}", path);
            }
        }
    }
    return extra_requires;
}

#[derive(Debug)]
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

    if build_and_compiler_type.to_string_lossy().contains("msbuild")
        || build_and_compiler_type.to_string_lossy().contains("cmake") 
        || build_and_compiler_type.to_string_lossy().contains("dist") {
        
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

#[derive(Debug)]
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
    if build_and_compiler_type.to_string_lossy().contains("msbuild")
        || build_and_compiler_type.to_string_lossy().contains("cmake") 
        || build_and_compiler_type.to_string_lossy().contains("dist") {
        
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
    if build_and_compiler_type.to_string_lossy().contains("msbuild")
        || build_and_compiler_type.to_string_lossy().contains("cmake") 
        || build_and_compiler_type.to_string_lossy().contains("dist") {
        
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
                        arg.to_string_lossy().contains(".c") || arg.to_string_lossy().contains(".cc") || arg.to_string_lossy().contains(".cxx")).collect::<Vec<std::ffi::OsString>>();
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

        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }

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
        
        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }

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

        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }

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
        
        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }
        
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
        
        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }
        
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
        
        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }
        
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

        let line = "C:\\PROGRA~1\\MICROS~2\\2022\\ENTERP~1\\VC\\Tools\\MSVC\\1437~1.328\\bin\\Hostx64\\x64\\cl.exe";
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
        
        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }
        
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

    #[test]
    fn parse_action_from_commands_test() {
        let compiler_commands = ["/c", "/I", "G:\\OpenSource\\llvm-project\\build\\lib\\Target\\PowerPC", "/Zi", "/nologo", "/W4", "/WX-", "/diagnostics:column", "/MP", "/Od", "/Ob0", "/Oi", "/D", "_UNICODE", "/D", "UNICODE", "/D", "WIN32", "/D", "_WINDOWS", "/D", "_HAS_EXCEPTIONS=0", "/D", "GTEST_HAS_RTTI=0", "/D", "LLVM_BUILD_STATIC", "/D", "_CRT_SECURE_NO_DEPRECATE", "/D", "_CRT_SECURE_NO_WARNINGS", "/D", "_SCL_SECURE_NO_WARNINGS", "/D", "UNICODE", "/D", "_UNICODE", "/D", "__STDC_CONSTANT_MACROS", "/D", "__STDC_FORMAT_MACROS", "/D", "__STDC_LIMIT_MACROS", "/D", "CMAKE_INTDIR=\\\"Debug\\\"", "/Zc:preprocessor", "/Gm-", "/RTC1", "/MDd", "/GS", "/fp:precise", "/Zc:wchar_t", "/Zc:forScope", "/Zc:inline", "/GR-", "/std:c++17", "/permissive-", "/FoLLVMExegesisTests.dir\\Debug\\/X86/BenchmarkResultTest.cpp.obj", "/FdLLVMExegesisTests.dir\\Debug\\vc143.pdb", "/external:W4", "/Gd", "/TP", "/wd4141", "/wd4146",  "/wd4204", "/wd4577", "/wd4091", "/wd4592", "/wd4319", "/wd4709", "/errorReport:prompt", "/we4238", "/bigobj", "-w14062", "/Gw", "/EHs-c-", "G:\\OpenSource\\llvm-project\\llvm\\unittests\\tools\\llvm-exegesis\\X86\\BenchmarkResultTest.cpp"].iter().map(|item|std::ffi::OsString::from(item)).collect::<Vec<_>>();
        let actions = parse_action_from_commands(&std::ffi::OsString::from("msbuild"), &compiler_commands, &std::ffi::OsString::from("G:\\OpenSource\\llvm-project\\build\\unittests\\tools\\llvm-exegesis"));
        println!("[parsed actions: {:?}", actions);
    }
    #[test]
    fn start_local_clang_cl_test() {
        
        let compiler_path = "..\\..\\third_party\\llvm-build\\Release+Asserts\\bin\\clang-cl.exe";
        let working_dir = "G:\\Chromium\\chromium\\src\\out\\Default";


        let mut process = std::process::Command::new(compiler_path);


        let child = process.current_dir(working_dir)    
                                .stdout( std::process::Stdio::piped())
                                .stderr( std::process::Stdio::piped())
                                .spawn();
        
        match child {
            Ok(mut child) => {
                let status = child.wait().expect("failed to wait on child");
                let code = status.code();
                println!("status code: {:?}", code);
            },
            Err(e) => {
                println!("failed to start process: {}", e);
            }
        }
    }

    #[test]
    fn test_zip_separate_file() {
        let (contents, path) = crate::communicate::packager::Packager::pack_separate_file("G:\\Chromium\\chromium\\src\\out\\Default\\../../build/config/warning_suppression.txt");
        
        let dir = "D:\\turbobuild\\target\\debug\\Replica\\Project\\Default";
        let cursor = std::io::Cursor::new(contents);
        let mut zip = zip::ZipArchive::new(cursor).unwrap();
        zip.file_names()
            .for_each(|name| println!("zip file name: {}", name));

        match zip.extract(dir) {
            Ok(_) => {
                println!("extract zip file done: {} {:?}", dir, zip.file_names().collect::<Vec<&str>>());
            },
            Err(err) => {
                println!("extract zip file failed. {} {} {:?}", dir, err, zip.file_names().collect::<Vec<&str>>());
            }
        }
    }

    #[test]
    fn test_local_compile_file_use_project_arg() {
        tools::logger::init_once_logger();
        let mut compiler_commands: Vec<std::ffi::OsString> = vec!["/c", "/I", "G:\\CommonWorkSpace\\webex\\common-head\\Services\\WebExCrossLaunch\\desktop", "/I", "G:\\CommonWorkSpace\\webex\\common-head", "/I", "G:\\CommonWorkSpace\\webex", "/I", "G:\\CommonWorkSpace\\webex\\common-head\\Utils", "/I", "G:\\CommonWorkSpace\\webex\\common-head\\ConnectivityBanner", "/I", "G:\\CommonWorkSpace\\webex\\common-head\\ViewModels", 
        "/I", "G:\\CommonWorkSpace\\webex\\common-head\\Services", "/I", "G:\\CommonWorkSpace\\webex\\common-head\\visuals", 
        "/I", "G:\\CommonWorkSpace\\webex\\common-head\\Meetings\\Utils", "/I", "G:\\CommonWorkSpace\\webex\\common-head\\Meetings\\MeetingDetails", 
        "/I", "G:\\CommonWorkSpace\\webex\\build_x64\\common-head", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Utilities\\WebexMeetingUtils", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\cjose\\source\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\jansson\\source\\build\\include", 
        "/I", "G:\\CommonWorkSpace\\webex\\build_x64\\ServiceFactories", "/I", "G:\\CommonWorkSpace\\webex\\ServiceFactories", 
        "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\webexcrosslaunch\\include", 
        "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\webexcrosslaunch\\MeetingCommunicator\\vendors\\internal\\jcf\\jcfcoreutils\\include", 
        "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\webexcrosslaunch\\MeetingCommunicator\\vendors\\internal\\jcf\\csf-foundation\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\webexcrosslaunch\\MeetingCommunicator\\vendors\\internal\\CommonUtils", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\webexcrosslaunch\\MeetingCommunicator\\vendors\\internal\\CWSSDK", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\webex\\include", 
        "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\AuxiliaryDeviceService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\AppExtensionService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\CalendarService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\CallHistoryService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\ContactService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\ConversationService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\CoreFramework", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\DynamicDependenciesService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\ECMService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\EncryptionService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\FeedbackService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\HighlightService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\ImageService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\MeetingRecordingService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\MediaEncryptionService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\MeetingContainerService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\OfficeService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\OnboardingService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\PersonalInsightsService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\PresenceService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\SearchService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\TeamService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\TelemetryService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\TelephonyService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\TestRestApiService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\UCLoginService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\UpgradeService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\UserGuidanceService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\VoicemailService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\WebexMeetingService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\WhiteboardService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\EdiscoveryService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\SmsService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\SpaceService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services\\RaindropService", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\BroadWorksCalling", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\BroadWorksCalling\\xsi", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\FeatureSettings", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\MediaEngine", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Services", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\Utilities", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\jabber\\ecc\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\virtual-channel-sdk\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\casablanca\\source\\Release\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\jabber\\ecc\\include\\dependent", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework", "/I", "G:\\CommonWorkSpace\\webex\\build_x64\\spark-client-framework", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\casablanca\\source\\Release\\libs\\websocketpp", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\Eigen", "/I", "G:\\CommonWorkSpace\\webex\\build_x64\\FeatureSettingsInjector", "/I", "G:\\CommonWorkSpace\\webex\\FeatureSettingsInjector", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\nlohmann\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\wme\\win\\include", "/I", "G:\\CommonWorkSpace\\webex\\spark-client-framework\\thirdparty\\libyuv\\source\\include", "/I", "G:\\CommonWorkSpace\\webex\\build_x64\\common-head\\visuals", "/I", "G:\\CommonWorkSpace\\webex\\build_x64\\common-head\\visuals\\autogen", "/ZI", "/nologo", "/W3", "/WX", "/diagnostics:column", "/MP", "/Od", "/Ob1", "/D", "_UNICODE", "/D", "UNICODE", "/D", "WIN32", "/D", "_WINDOWS", "/D", "_REPORT_PPLTASK_UNOBSERVED_EXCEPTION=void", "/D", "_SILENCE_STDEXT_ARR_ITERS_DEPRECATION_WARNING", "/D", "COMMON_HEAD_STATIC_DEFINE", "/D", "NOMINMAX", 
        "/D", "DEBUG", "/D", "_DEBUG", "/D", "UNICODE", "/D", "_UNICODE", "/D", "CMAKE_BUILD", "/D", "WEBEXMEETINGUTILS_STATIC_DEFINE", "/D", "SCF_STATIC_DEFINE", 
        "/D", "CJOSE_STATIC_DEFINE", "/D", "JANSSON_STATIC_DEFINE", "/D", "SERVICEFACTORIES_STATIC_DEFINE", "/D", "MEETINGCOMMUNICATOR_STATIC_DEFINE", 
        "/D", "CROSSLAUNCH_JCF_STATIC_DEFINE", "/D", "CROSSLAUNCH_COMMONUTILS_STATIC_DEFINE", "/D", "CROSSLAUNCH_CWSSDK_STATIC_DEFINE", 
        "/D", "MEETINGMANAGERSDK_STATIC_DEFINE", "/D", "COMMON_HEAD_UTILS_STATIC_DEFINE", "/D", "CPPREST_STATIC_DEFINE", "/D", "_HAS_AUTO_PTR_ETC=1", "/D", "_NO_ASYNCRTIMP", "/D", "_LIBCPP_ENABLE_CXX17_REMOVED_AUTO_PTR=1", "/D", "GUIDUTILITIES_STATIC_DEFINE", "/D", "_SILENCE_CXX17_NEGATORS_DEPRECATION_WARNING", "/D", "PUBLISHINGUTILITIES_STATIC_DEFINE", "/D", "JWTUTILITIES_STATIC_DEFINE", "/D", "FEATURESETTINGSINJECTOR_STATIC_DEFINE", "/D", "NETWORKUTILITIES_STATIC_DEFINE", "/D", "TELEMETRYUTILS_STATIC_DEFINE", "/D", "QT_WIDGETS_LIB", "/D", "QT_GUI_LIB", "/D", "QT_CORE_LIB", "/D", "QT_QUICK_LIB", "/D", "QT_QMLMODELS_LIB", "/D", "QT_QML_LIB", "/D", "QT_NETWORK_LIB", "/D", "QT_QUICKWIDGETS_LIB", "/D", "VISUALS_STATIC_DEFINE", 
        "/D", "CMAKE_INTDIR=\\\"Debug\\\"", "/Gm-", "/EHsc", "/RTC1", "/MDd", "/GS", "/fp:precise", "/Qspectre", "/Zc:wchar_t", "/Zc:forScope", "/Zc:inline", 
        "/GR", "/std:c++17", "/permissive-", "/Focommon-head.dir\\Debug\\", "/FdG:\\CommonWorkSpace\\webex\\build_x64\\output\\lib\\Debug\\common-head.pdb", "/external:W0", "/Gd", "/TP", "/wd5286", "/wd4267", "/wd4244", "/wd4018", "/wd4715", "/wd4834", "/wd4996", "/errorReport:prompt", "/we4700", "/we4701", "/we6001", "/we26494", "/external:I", "G:/CommonWorkSpace/webex/spark-client-framework/thirdparty/ciscossl/win-x64-release/shared/include", "/external:I", "G:/CommonWorkSpace/webex/spark-client-framework/thirdparty/boost/windows/include", "/external:I", "G:/CommonWorkSpace/webex/common-head/NativeWhiteboard/libwhiteboardingv2", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtWidgets", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtGui", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtANGLE", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtCore", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/./mkspecs/win32-msvc", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtQuick", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtQmlModels", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtQml", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtNetwork", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtQuickWidgets", "/bigobj", "/utf-8", "/F2000000", 
        "G:\\CommonWorkSpace\\webex\\common-head\\jssdk\\JSSDKEventSubscriptionHelper.cpp"
        ].iter().map(|item|std::ffi::OsString::from(*item)).collect::<Vec<std::ffi::OsString>>();

        let current_crate_dir = "G:\\CommonWorkSpace\\webex\\build_x64\\common-head\\common-head.dir\\Debug";
        let current_crate_dir = std::path::PathBuf::from(current_crate_dir); 
        let env = crate::platform::windows::WindowsCompilerEnv::default();
        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        for include in &env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", include.to_string_lossy())));
        }
        
        for sdk_include in env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from("/I"));
            compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
        }

        let _ = request_local_compile(&complier_path.into_os_string(), &current_crate_dir.into_os_string(), &compiler_commands, std::ffi::OsString::from(""), false);
    }

}