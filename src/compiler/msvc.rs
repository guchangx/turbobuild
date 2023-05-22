extern crate regex;
pub struct MSVC;
use std::{ops::{Index, Add}, io::Write};

#[async_trait]
impl crate::compiler::compiler::Compiler for MSVC {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                                compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle,
                                grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>)
                                -> (super::compiler::CompileOutput, Option<super::compiler::ProcessedResults>) {
        let grade = grade.lock().unwrap().fetch_grade();
        let output = request_msvc_compile(working_parameters, compile_input, pool, grade).await;
        return output;
    }

    async fn dist_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
            compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle,
            grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>)
                                    -> (super::compiler::CompileOutput, Option<super::compiler::ProcessedResults>) {
        
        let mut compiler_env = crate::platform::windows::WindowsCompilerEnv::default();

        if false {
            match compile_input.env_input.clone() {
                Some(env) => {
                    let mut winkits_includes_path: Vec<std::string::String> = Vec::new();
                    for path in env.winkits_includes_path {
                        let path = path.to_str().unwrap().to_owned();
                        winkits_includes_path.push(path);
                    }
                    compiler_env.winkits_includes_path = winkits_includes_path;
                    compiler_env.compiler_path = std::path::PathBuf::from(env.compiler_path);
                    compiler_env.msvc_includes_path = std::path::PathBuf::from(env.msvc_includes_path);
                },
                None => {
    
                },
            }
            let params = crate::buildturbo::WorkingParameters {
                storage: working_parameters.storage,
                dist: working_parameters.dist,
                compiler_env: compiler_env,
                network_client: working_parameters.network_client,
            };
            let grade = grade.lock().unwrap().fetch_grade();
            let output = request_msvc_compile(params, compile_input, pool, grade).await;
            return output;
        }
        else {
            let output = request_local_compile_by_preprocessed_source(&compile_input, pool);
            return output;
        }
    }

    async fn remote_request_compile(&self, _working_parameters: crate::buildturbo::WorkingParameters,
                                    compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle, 
                                    _grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>)
                                    -> (super::compiler::CompileOutput, Option<super::compiler::ProcessedResults>) {
        let (output, results) = request_local_compile_by_preprocessed_source(&compile_input, pool);
        return (output, results);
    }
}

async fn request_msvc_compile(working_parameters: crate::buildturbo::WorkingParameters,
                                msvc_compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle,
                                grade: crate::utils::grade::LocalGrade) 
                                -> (super::compiler::CompileOutput, Option<super::compiler::ProcessedResults>) {

    let storage = working_parameters.storage;
    let env = working_parameters.compiler_env;
    let now = std::time::Instant::now();
    let (mut compiler_commands, compiler_path) = parse_compiler_input_command(msvc_compile_input.clone(), &env);
    println!("parse compiler input commmands elaspsed time:{:?}", now.elapsed());
    let working_path = std::path::PathBuf::from(msvc_compile_input.compiler_working_dir.to_string_lossy().to_string());

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

    let source_file = fetch_compiler_source_file(msvc_compile_input.build_and_compiler_type.clone(), 
        compiler_commands.clone(), working_path.clone()).unwrap();


    let object = fetch_compiler_object_file(msvc_compile_input.build_and_compiler_type.clone(), 
                                compiler_commands.clone(), working_path.clone());

    let mut need_compile_file_key: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if false && storage.is_some() {
        for (file, path) in source_file {
        
            let key = crate::utils::hasher::Digest::file(path.clone(), pool).await;
            
            match key {
                Ok(key) => {
                    let storage = storage.clone().unwrap();
                    let exist = exist_source_file_generated(&key, &storage).await;
                    if exist {
                        exempt_compile_current_source_by_cache(file.clone(), &mut compiler_commands);

                        match &object {
                            GeneratedObject::PathWithObjName(object_filepath_with_filename) => {
                                get_generated_cache_from_storage(&key, object_filepath_with_filename, &storage).await;
                            },
                            GeneratedObject::PathWithoutObjName(object_filepath_without_filename) => {
                                let suffix_index = file.rfind('.').unwrap();
                                let mut filename = file.clone();
                                let _ = filename.split_off(suffix_index);
                                filename.push_str(".obj");

                                let object_path = object_filepath_without_filename.join(filename);
                                get_generated_cache_from_storage(&key, &object_path, &storage).await;
                            },
                            _ => {
                                println!("get cache operate can't fetch object file path.");
                            },
                        }
                        
                    }
                    else {
                        println!("compile source file name: {:?}, path: {:?}, key: {:?}", file, path.clone(), key);
                        need_compile_file_key.insert(file, key);
                    }
                },
                Err(error) => {
                    println!("file can't get hasher key. error info: {:?}", error);
                },
            }
        }
    }

    let exists_source_file_in_command = determine_whether_need_compile(compiler_commands.clone());
    
    if exists_source_file_in_command {
        let mut default_output = super::compiler::CompileOutput::default();
        let mut default_results = Vec::<super::compiler::ProcessedResult>::new();

        if grade.cpu_usage < 00.0 {
            let (output, results) = request_local_compile(compiler_path, msvc_compile_input.compiler_working_dir, compiler_commands.clone(), msvc_compile_input.build_and_compiler_type, false);
            default_output.set(output);
            if let Some(results) =  results {
                default_results = results;
            }
        }
        else {
            if true {
                // dist with preprocessed source
                let now = std::time::Instant::now();
                let output = request_dist_multi_sync_once_compile(&working_parameters.network_client, &compiler_path, &msvc_compile_input.compiler_working_dir, &compiler_commands.clone());
                println!("request_dist_multi_sync_once_compile elaspsed time:{:?}", now.elapsed());
                //let output = request_dist_compile(&working_parameters.network_client, &compiler_path, &msvc_compile_input.compiler_working_dir, &compiler_commands.clone());
                default_output.set(output);
            }
            else {
                //dist with source file and include file
                let parameters =  crate::buildturbo::WorkingParameters {
                    storage: storage.clone(),
                    dist: working_parameters.dist,
                    compiler_env: env,
                    network_client: working_parameters.network_client,
                };

                let output = request_dist_compile_with_source_and_include(&parameters, &msvc_compile_input);
                default_output.set(output);
            }
        }

        if default_output.compile_status && storage.is_some() {
            for compiled_file in default_output.compiled_filename.clone() {
                let compiled_file = compiled_file.to_str().unwrap();
                let key = need_compile_file_key.get(compiled_file);
                match key {
                    Some(key) => {

                        match &object {
                            GeneratedObject::PathWithObjName(object_filepath_with_filename) => {
                                let storage = storage.clone().unwrap();
                                set_generated_cache_to_storage(&key, object_filepath_with_filename, &storage).await;
                            },
                            GeneratedObject::PathWithoutObjName(object_filepath_without_filename) => {
                                let suffix_index = compiled_file.rfind('.').unwrap();
                                let mut filename = compiled_file.to_owned();
                                let _ = filename.split_off(suffix_index);
                                filename.push_str(".obj");
                                let object_path = object_filepath_without_filename.join(filename);
                                let storage = storage.clone().unwrap();
                                set_generated_cache_to_storage(&key, &object_path, &storage).await;
                            },
                            _ => {
                                println!("set cache can't fetch object file path.");
                            },
                        }
                    },
                    None => {
                        println!("don't get key in HashMap<filename, key> by source filename. {:?}", compiled_file);
                    },
                }
            }
            
        }
        else {
            
        }
        if default_results.is_empty() {
            return (default_output, None);
        }
        else {
            return (default_output, Some(default_results))
        }        
    }
    else {
        let compiled_filename: Vec<std::ffi::OsString> = Vec::new();
        let result = super::compiler::CompileOutput {
            compiled_filename,
            compile_status: false,
            compile_output: std::ffi::OsString::from("don't need compile anything."),
        };
        return (result, None);
    }
}

fn request_dist_compile(network: &crate::network::client::NetworkClient, compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
    compiler_commands: &Vec<std::ffi::OsString>) -> super::compiler::CompileOutput {

    let result = std::sync::Arc::new(std::sync::Mutex::new(super::compiler::CompileOutput::default()));
    let now = std::time::Instant::now();
    let (status, stdout, stderr) = request_local_precompile(compiler_path, compiler_working_dir, compiler_commands);
    if status {
        let files = String::from_utf8_lossy(&stderr);

        let mut commands:Vec<std::ffi::OsString> = Vec::new();
        let mut source_files: Vec<String> = Vec::new();
        let mut pdb = String::from("");
        let mut project = String::from("");

        for (_, value) in compiler_commands.iter().enumerate() {
            let value = value.to_string_lossy();
            if value.ends_with(".cpp") {
                source_files.push(value.to_string());
                continue;
            }
            else if value.ends_with(".c") {
                source_files.push(value.to_string());
                continue;
            }
            else if value.starts_with("/Fd") {
                // Fd"abi_compat_inline_ns.dir\Debug\vc143.pdb"
                pdb = value.to_string();
                continue;
            }
            else if value.contains(".dir") {
                if project.is_empty() {
                    project = value.to_string();
                }
                commands.push(std::ffi::OsString::from(value.to_string()));
                continue;
            }
            else {
                commands.push(std::ffi::OsString::from(value.to_string()));
            }
        };

        let mut content = String::from_utf8_lossy(&stdout);
        let count = files.lines().count();
        if count.eq(&source_files.len()) {
            log::trace!("preprocessed multiple sources, count: {:?}. elapsed time: {:?}.", count, now.elapsed());
        }
        else {
            log::warn!("preprocessed multiple sources are not same with commands");
        }
        log::trace!("preprocessed source file {:?}", files);
       
        let mut index = 0;
        let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
        for file in &source_files {
            
            let mut split = commands.clone();
            let path = std::path::PathBuf::from(file);
            let source_name = match path.file_stem() {
                Some(value) => {
                    value.to_string_lossy().add(".pdb")
                },
                None => {
                    std::borrow::Cow::from("bt_1.pdb")
                }
            };
            
            if pdb.ends_with(".pdb") {
                let original_pdb_name = pdb.split("\\").filter(|value| value.contains(".pdb")).collect::<String>();
                let pdb = pdb.replace(&original_pdb_name, &source_name);
                split.push(std::ffi::OsString::from(pdb));
            }

            let i_path = push_project_name_to_precompiled_file_path(&path, project.clone());
            split.push(std::ffi::OsString::from(&i_path));
            
            if let Some(next_file) = source_files.get(index + 1) {
                //#line 1 "D:\\TrainSpace\\json\\tests\\abi\\main.cpp"
                let line = format!(r#"#line 1 "{}""#, next_file).replace(r"\", r"\\");
                if let Some(position) = content.find(&line) {
                    if position.gt(&0) {
    
                        let (first, last) = content.split_at(position);
            
                        let msvc_compile_input = super::compiler::CompileInput {
                            compiler_path_or_arch: compiler_path.to_owned(),
                            compiler_working_dir: compiler_working_dir.to_owned(),
                            compiler_commands: split.to_owned(),
                            build_and_compiler_type: std::ffi::OsString::from("MSBuild Precompile"),
                            env_input: None
                        };
                        
                        let precompiled_suorce = super::compiler::PrecompiledSource {
                            preprocessed_source_contents: Some(first.as_bytes().to_vec()),
                            preprocessed_source_path: std::ffi::OsString::from(&i_path)
                        };
                        log::debug!("precompile {:?} source file. precompiled result size: {:.2?}M.", i_path.file_name().unwrap(), first.as_bytes().len() as f32 / 1024.0 / 1024.0);

                        content = last.to_string().into();
                        let net = network.clone();
                        let result = result.clone();
                        let handle = std::thread::spawn(move || {
                            let output = request_dist_compile_and_sync_result(&net, &msvc_compile_input, &precompiled_suorce);
                            let mut result = result.lock().unwrap();

                            let mut file = output.compiled_filename;
                            result.compiled_filename.append(&mut file);
                            let mut compile_output = result.compile_output.to_string_lossy().to_string();
                            compile_output.push_str(output.compile_output.to_str().unwrap());
                            result.compile_output = std::ffi::OsString::from(compile_output);
                            result.compile_status = output.compile_status;
                        });
                        handles.push(handle);
                    }
                }
            }
            else {
                let msvc_compile_input = super::compiler::CompileInput {
                    compiler_path_or_arch: compiler_path.to_owned(),
                    compiler_working_dir: compiler_working_dir.to_owned(),
                    compiler_commands: split.to_owned(),
                    build_and_compiler_type: std::ffi::OsString::from("MSBuild Precompile"),
                    env_input: None
                };
                let precompiled_suorce = super::compiler::PrecompiledSource {
                    preprocessed_source_contents: Some(content.as_bytes().to_vec()),
                    preprocessed_source_path: std::ffi::OsString::from(&i_path)
                };

                log::debug!("precompile {:?} source file. precompiled result size: {:.2?}M.", path.file_name().unwrap(), content.as_bytes().len() as f32 / 1024.0 / 1024.0);
                let now = std::time::Instant::now();
                let output = request_dist_compile_and_sync_result(network, &msvc_compile_input, &precompiled_suorce);
                log::debug!("request {:?} dist compile with precompiled source response elapsed: {:?}.", path.file_name().unwrap(), now.elapsed());
                let mut result = result.lock().unwrap();

                let mut file = output.compiled_filename;
                result.compiled_filename.append(&mut file);
                let mut compile_output = result.compile_output.to_string_lossy().to_string();
                compile_output.push_str(output.compile_output.to_str().unwrap());
                result.compile_output = std::ffi::OsString::from(compile_output);
                result.compile_status = output.compile_status;
            }

            index = index.add(1);
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let result = result.lock().unwrap();
        return result.clone();
    }
    else {
        let mut result = result.lock().unwrap();
        result.compile_status = false;
        let output = String::from_utf8_lossy(&stderr);
        result.compile_output = std::ffi::OsString::from(output.to_string());
        println!("preprocessed source file failed. {:?}", output);
    }
    let result = result.lock().unwrap();
    return result.clone();
}

fn request_dist_multi_sync_once_compile(network: &crate::network::client::NetworkClient, compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
    compiler_commands: &Vec<std::ffi::OsString>) -> super::compiler::CompileOutput {

    let mut result = super::compiler::CompileOutput::default();
    let now = std::time::Instant::now();
    let (status, stdout, stderr) = request_local_precompile(compiler_path, compiler_working_dir, compiler_commands);
    if status {
        let files = String::from_utf8_lossy(&stderr);
        
        let mut commands:Vec<std::ffi::OsString> = Vec::new();
        let mut source_files: Vec<String> = Vec::new();
        let mut project = String::from("");

        for (_, value) in compiler_commands.iter().enumerate() {
            let value = value.to_string_lossy();
            if value.ends_with(".cpp") {
                source_files.push(value.to_string());
                continue;
            }
            else if value.ends_with(".c") {
                source_files.push(value.to_string());
                continue;
            }
            else if value.contains(".dir") {
                if project.is_empty() {
                    project = value.to_string();
                }
                commands.push(std::ffi::OsString::from(value.to_string()));
                continue;
            }
            else {
                commands.push(std::ffi::OsString::from(value.to_string()));
            }
        };
        let mut content = String::from_utf8_lossy(&stdout);
        let count = files.lines().count();
        if count.eq(&source_files.len()) {
            log::trace!("preprocessed multiple sources, count: {:?}. elapsed time: {:?}.", count, now.elapsed());
        }
        else {
            log::warn!("preprocessed multiple sources are not same with commands. elapsed time: {:?}.", now.elapsed());
        }

        log::trace!("preprocessed source file {:?}", files);
       
        let mut index = 0;
        let mut handles = Vec::<std::thread::JoinHandle<()>>::new();

        for file in &source_files {
            
            let path = std::path::PathBuf::from(file);
            let extension_i_path = push_project_name_to_precompiled_file_path(&path, project.clone());
            commands.push(extension_i_path.clone().into_os_string());
            
            if let Some(next_file) = source_files.get(index + 1) {
                //#line 1 "D:\\TrainSpace\\json\\tests\\abi\\main.cpp"
                let line = format!(r#"#line 1 "{}""#, next_file).replace(r"\", r"\\");
                if let Some(position) = content.find(&line) {
                    if position.gt(&0) {
    
                        let (first, last) = content.split_at(position);
            
                        let msvc_compile_empty_input = super::compiler::CompileInput {
                            compiler_path_or_arch: compiler_path.to_owned(),
                            compiler_working_dir: std::ffi::OsString::from(""),
                            compiler_commands: Vec::<std::ffi::OsString>::new(),
                            build_and_compiler_type: std::ffi::OsString::from("Sync Precompiled Source File"),
                            env_input: None
                        };

                        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(vec![]));
                        let options = zip::write::FileOptions::default()
                                                .compression_method(zip::CompressionMethod::Zstd);
                        let name = extension_i_path.file_name().unwrap();
                        zip.start_file(name.to_string_lossy(), options).unwrap();
                        zip.write_all(first.as_bytes()).unwrap();
                        let result = zip.finish().unwrap();

                        let precompiled_suorce = super::compiler::PrecompiledSource {
                            preprocessed_source_contents: Some(result.get_ref().to_vec()),
                            preprocessed_source_path: std::ffi::OsString::from(&extension_i_path)
                        };

                        log::debug!("sync precompiled source file {:?}. size: {:.2?}M.", extension_i_path.file_name().unwrap(), first.as_bytes().len() as f32 / 1024.0 / 1024.0);
                        content = last.to_string().into();
                        let net = network.clone();
                        let handle = std::thread::spawn(move || {
                            let _ = request_dist_compile_and_sync_result(&net, 
                                &msvc_compile_empty_input, &precompiled_suorce);
                        });
                        
                        handles.push(handle);
                    }
                }
            }
            else {

                let msvc_compile_input = super::compiler::CompileInput {
                    compiler_path_or_arch: compiler_path.to_owned(),
                    compiler_working_dir: compiler_working_dir.to_owned(),
                    compiler_commands: commands.to_owned(),
                    build_and_compiler_type: std::ffi::OsString::from("MSBuild Precompile"),
                    env_input: None
                };

                let mut cursor = std::io::Cursor::new(Vec::new());
                let mut zip = zip::ZipWriter::new(&mut cursor);
                let options = zip::write::FileOptions::default()
                                        .compression_method(zip::CompressionMethod::Zstd);
                let name = extension_i_path.file_name().unwrap();
                zip.start_file(name.to_string_lossy(), options).unwrap();
                zip.write_all(content.as_bytes()).unwrap();
                let zip_content = zip.finish().unwrap();

                let precompiled_suorce = super::compiler::PrecompiledSource {
                    preprocessed_source_contents: Some(zip_content.get_ref().to_vec()),
                    preprocessed_source_path: std::ffi::OsString::from(&extension_i_path)
                };

                for handle in handles {
                    handle.join().expect("join sync precompiled results thread failed.");
                }

                let now = std::time::Instant::now();
                let output = request_dist_compile_and_sync_result(network, &msvc_compile_input, &precompiled_suorce);
                log::debug!("request {:?} dist compile with precompiled source size: {:?}M, response elapsed: {:?}.", extension_i_path.file_name().unwrap(), content.as_bytes().len() as f32 / 1024.0 / 1024.0, now.elapsed());
        
                let mut file = output.compiled_filename;
                result.compiled_filename.append(&mut file);
                result.compile_output = std::ffi::OsString::from(output.compile_output.to_str().unwrap());
                result.compile_status = output.compile_status;

                break;
            }
            index = index.add(1);
        }
        return result;
    }
    else {
        return result;
    }
}

fn request_local_precompile(compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
    compiler_commands: &Vec<std::ffi::OsString>) -> (bool, std::rc::Rc<Vec<u8>>, std::rc::Rc<Vec<u8>>) {

    let mut commands = compiler_commands.to_owned();
    commands.insert(0, std::ffi::OsString::from(r"/E"));
    //remove '/MP'. /E incompatible with multiprocessing
    commands.retain(|item| !item.to_string_lossy().starts_with("/MP"));

    let (status, stdout, stderr) = start_local_compiler(compiler_path, compiler_working_dir, &commands);
    return (status, stdout, stderr);
}

fn push_project_name_to_precompiled_file_path(path: &std::path::PathBuf, project: String) -> std::path::PathBuf
{
    if !project.is_empty() && project.contains(".dir") {
        let index = project.find(".dir").unwrap();
        let (first, _)= project.split_at(index);
        let (_, project_name) = first.split_at(3);
        
        let mut path = path.to_owned();
        path.set_extension("i");
        if let Some(precompile_file_name) = path.file_name() {
            if let Some(parent) = path.parent() {
                let mut path = parent.to_path_buf();
                path.push(project_name);
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

fn request_dist_compile_and_sync_result(network: &crate::network::client::NetworkClient, msvc_compile_input: &super::compiler::CompileInput, precompiled_source: &super::compiler::PrecompiledSource)
    -> super::compiler::CompileOutput {
    let result = request_dist_compile_with_precompiled_source(&network, &msvc_compile_input, &precompiled_source);
    if result.compile_status {
    }
    else {
        log::trace!("request remote compile and sync back failed: {:?}", result);
    }
    return result;
}

fn request_local_compile(compiler_path: std::ffi::OsString, compiler_working_dir: std::ffi::OsString, 
                                compiler_commands: Vec<std::ffi::OsString>, build_and_compiler_type: std::ffi::OsString,
                            sync_compile_result: bool) -> (super::compiler::CompileOutput, Option<Vec<super::compiler::ProcessedResult>>) {
    let now = std::time::Instant::now();
    let (status, stdout, _stderr) = start_local_compiler(&compiler_path, &compiler_working_dir, &compiler_commands);
    let compile_output = String::from_utf8_lossy(&stdout);
    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    let mut compiled_results: Vec<crate::compiler::compiler::ProcessedResult> = Vec::new();

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
        let working_path = std::path::PathBuf::from(compiler_working_dir.to_owned());
        let (pdb_path, _one_pdb )= fetch_compile_pdb_path(build_and_compiler_type.clone(), compiler_commands.to_owned(), working_path.clone());
        let pdb_path = std::rc::Rc::new(pdb_path);
        for line in output {
            let line = line.replace(r#"""#, "");
            if line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".i") {
                if sync_compile_result {
                    let mut obj: Option<(std::ffi::OsString, Vec<u8>)> = None;
                    let mut pdb: Option<(std::ffi::OsString, Vec<u8>)> = None;
                    let mut idb: Option<(std::ffi::OsString, Vec<u8>)> = None;
    
                    let mut result_path = std::path::PathBuf::from("");
                    let object = fetch_compiler_object_file(build_and_compiler_type.clone(), compiler_commands.to_owned(), working_path.clone());
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
                            let mut encode = Vec::new();
                            zstd::stream::copy_encode(file, &mut encode, 6).unwrap();
                            obj = Some((std::ffi::OsString::from(result_path.to_str().unwrap()), encode));
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
                                let mut encode = Vec::new();
                                zstd::stream::copy_encode(file, &mut encode, 6).unwrap();
                                pdb = Some((std::ffi::OsString::from(result_path.to_str().unwrap()), encode));
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
                                let mut encode = Vec::new();
                                zstd::stream::copy_encode(file, &mut encode, 6).unwrap();
                                idb = Some((std::ffi::OsString::from(result_path.to_str().unwrap()), encode));
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

                    let processed_result = crate::compiler::compiler::ProcessedResult {
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
    
    let result = super::compiler::CompileOutput {
        compiled_filename,
        compile_status: status,
        compile_output: std::ffi::OsString::from(compile_output.to_string()),
    };

    return (result, Some(compiled_results));
}

fn request_local_compile_by_preprocessed_source(msvc_compile_input: &super::compiler::CompileInput, _pool: &tokio::runtime::Handle) -> (super::compiler::CompileOutput, Option<super::compiler::ProcessedResults>) {

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

    let (output, results) = request_local_compile(msvc_compile_input.compiler_path_or_arch.clone(),
                    msvc_compile_input.compiler_working_dir.clone(), commands,
                    msvc_compile_input.build_and_compiler_type.clone(), true);

    return (output, results);
}

fn request_dist_compile_with_source_and_include(working_parameters: &crate::buildturbo::WorkingParameters, msvc_compile_input: &super::compiler::CompileInput) -> super::compiler::CompileOutput {
    log::debug!("request dist compile with source and include file.");
    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
    let local_compiler_arch = msvc_compile_input.compiler_path_or_arch.to_str().unwrap();
    let compiler_dir = std::path::Path::new(&working_parameters.compiler_env.compiler_path).join("Hostx64").join(local_compiler_arch);

    log::trace!("vs compiler install dir: {:?}.", compiler_dir);

    let winsdk_path = working_parameters.compiler_env.winkits_includes_path.first().unwrap();
    
    let sender = crate::syncfile::sender::Sender::new(&working_parameters.network_client);

    let mut dist_msvc_compiler_path: std::ffi::OsString;
    let mut dist_msvc_include_path: std::ffi::OsString;
    let mut win_kits_include_dir: std::ffi::OsString;

    (dist_msvc_compiler_path, dist_msvc_include_path, win_kits_include_dir) = sender.dist_kits_and_tool_pre_sync(winsdk_path, compiler_dir.to_str().unwrap());

    if dist_msvc_compiler_path.is_empty() {
        log::debug!("msvc toolchain sync");
        let path = std::path::PathBuf::from(compiler_dir.to_str().unwrap());
        if path.is_dir() {
            (dist_msvc_compiler_path, dist_msvc_include_path) = sender.sync_toolchain(&path);
        }
    }
    else {
        log::debug!("tool chain sync have done");
    }

    if win_kits_include_dir.is_empty() {
        log::debug!("windows kits sync");
        let mut path = std::path::PathBuf::from(winsdk_path);
        if path.is_dir() && path.pop() {
            win_kits_include_dir = sender.sync_windows_kits(&path);
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

    let env_input = crate::compiler::compiler::EnvInput {
         winkits_includes_path: vec![win_kits_include_dir],
         compiler_path: dist_msvc_compiler_path.clone(),
         msvc_includes_path: dist_msvc_include_path,
         msvc_version: std::ffi::OsString::new(),
         env_args: std::ffi::OsString::new(),
    };

    let mut input = msvc_compile_input.to_owned();
    input.env_input = Some(env_input.clone());
    input.build_and_compiler_type = std::ffi::OsString::from("MSBuild Dist");
    
    let response = sender.dist_compile_with_source_and_include(&input);

    return response;
}

fn request_dist_compile_with_precompiled_source(network: &crate::network::client::NetworkClient, msvc_compile_input: &super::compiler::CompileInput, precompiled_source: &super::compiler::PrecompiledSource) 
                                        -> super::compiler::CompileOutput {
    
    let now = std::time::Instant::now();
    let sender = crate::syncfile::sender::Sender::new(network);
    let mut dist_msvc_compiler_path= sender.dist_kits_and_tool_pre_sync("", msvc_compile_input.compiler_path_or_arch.to_str().unwrap()).0;
    if dist_msvc_compiler_path.is_empty() {

        let mut path = std::path::PathBuf::from(msvc_compile_input.compiler_path_or_arch.to_str().unwrap());

        if path.is_file() {
            path.pop();  
        }

        if path.is_dir() {
            (dist_msvc_compiler_path, _) = sender.sync_toolchain(&path);

            if !dist_msvc_compiler_path.is_empty() {
                let mut compiler_path = std::path::PathBuf::from(&dist_msvc_compiler_path);
                if !compiler_path.ends_with("cl.exe") {
                    compiler_path.push("cl.exe");
                    dist_msvc_compiler_path = std::ffi::OsString::from(compiler_path.to_str().unwrap());
                }
            }
        }
    }
    else {

    }
    log::debug!("fetch compiler toolchain and win kits response elapsed: {:?}", now.elapsed());

    let mut input = msvc_compile_input.to_owned();
    input.build_and_compiler_type = std::ffi::OsString::from("MSBuild Dist Precompile");
    input.compiler_path_or_arch = dist_msvc_compiler_path;

    let response = sender.dist_compile_with_precompiled_source(&input, &precompiled_source);
    return response;

}

fn start_local_compiler(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> (bool, std::rc::Rc<Vec<u8>>, std::rc::Rc<Vec<u8>>) {
    use std::process::Stdio;

    log::trace!("local compile working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compile content: {:?}", compiler_commands);

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
            let output = child.wait_with_output();
            match output {
                Ok(output) => {
                    if output.status.success() {
                        let elapsed = start.elapsed();
                        let mut output_context = String::from_utf8_lossy(&output.stderr);
                        if output.stderr.is_empty() {
                            output_context = String::from_utf8_lossy(&output.stdout);
                        }
                        log::trace!("local compile {:?} success, compiled elapsed time: {:?}, child thread Id: {:?}", output_context, elapsed, child_id);
                        return (true, std::rc::Rc::new(output.stdout), std::rc::Rc::new(output.stderr));
                    }
                    else {
                        let mut output_context = String::from_utf8_lossy(&output.stdout);
                        if output.stdout.is_empty() {
                            output_context = String::from_utf8_lossy(&output.stderr);
                        }
                        let elapsed = start.elapsed();
                        log::info!("compile file elapsed time: {:?}. error message: {:?}, error code: {:?}.", elapsed, output_context, output.status.code());
                        return (false, std::rc::Rc::new(output.stdout), std::rc::Rc::new(output.stderr));
                    }
                },
                Err(error) => {
                    log::warn!("compile child wait output error: {:?}", error);
                    let mut error_description = String::from("compile child wait output error: ");
                    error_description.push_str(error.to_string().as_str());
                    return (false, std::rc::Rc::new(error_description.into_bytes()), std::rc::Rc::new(vec![]));
                },
            }
        },
        Err(error) => {
            println!("spawn compile child process error: {:?}", error);
            let mut error_description = String::from("spawn compile child process error: ");
            error_description.push_str(error.to_string().as_str());
            return (false,  std::rc::Rc::new(error_description.into_bytes()), std::rc::Rc::new(vec![]));
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

fn parse_compiler_input_command(compile_input: super::compiler::CompileInput, working_compiler_env: &crate::platform::windows::WindowsCompilerEnv) -> (Vec<std::ffi::OsString>, std::ffi::OsString) {

    let compile_commands:Vec<std::ffi::OsString> = compile_input.compiler_commands;
    let mut args = Vec::new();
    let mut compiler_path = std::ffi::OsString::new();
    let commands_iter = compile_commands.iter().map(|arg| arg.to_str().unwrap().to_owned());
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSBuild") {
        let mut commands = commands_iter.last().unwrap();

        if compile_input.env_input.is_none() {
            let complier_path_from_input = extract_additional_input_commands(&mut commands);
            match complier_path_from_input {
                Some(compiler_path_specified) => {
                    compiler_path = compiler_path_specified;
                },
                None => {
                    //use default x64 cl.exe
                    // C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.32.31326\bin\Hostx64\x64\cl.exe
                    let path = &working_compiler_env.compiler_path;
                    let compiler_path_by_specified = path.join("Hostx64").join("x64").join("cl.exe");
                    compiler_path = std::ffi::OsString::from(compiler_path_by_specified.to_str().unwrap());
                },
            }
        }
        else {
            match compile_input.env_input {
                Some(env) => {
                    compiler_path = env.compiler_path;
                },
                None => {
                    
                },
            }
        }

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
            instruct += &include;
            args.push(std::ffi::OsString::from(instruct));
        }
    
        let msvc_includes = working_compiler_env.msvc_includes_path.display().to_string();
    
        let mut instruct = String::from("/I");
        instruct += &msvc_includes;
        args.push(std::ffi::OsString::from(instruct));
        
        return (args, compiler_path);
    }
    else if  compile_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
        compiler_path = compile_input.compiler_path_or_arch;
        return (args, compiler_path);
    }
    else {
        return (args, compiler_path);
    }
}

fn fetch_compiler_source_file(build_and_compiler_type: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>, working_dir: std::path::PathBuf) 
                                    -> Option<std::collections::HashMap<String, std::path::PathBuf>> {

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

enum GeneratedObject {
    NoneObjPath,
    PathWithObjName(std::path::PathBuf),
    PathWithoutObjName(std::path::PathBuf),
}

fn fetch_compiler_object_file(build_and_compiler_type: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>, working_dir: std::path::PathBuf) -> GeneratedObject {

    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake") 
        || build_and_compiler_type.to_string_lossy().contains("Dist") {

        let object_param = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().starts_with("/Fo")).collect::<Vec<_>>();

        match object_param.last() {
            Some(object) => {
                let object_path = object.to_string_lossy().to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
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
            },
            None => {
                println!("NoneObjPath NoneObjPath NoneObjPath");
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

fn fetch_compile_pdb_path(build_and_compiler_type: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>, working_dir: std::path::PathBuf) -> (ProgramDataBase, bool) {
    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake") 
        || build_and_compiler_type.to_string_lossy().contains("Dist") {

        let pdb_param = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().starts_with("/Fd")).collect::<Vec<_>>();
        
        //"/FdD:\\TrainSpace\\json\\Build\\tests\\abi\\diag\\Debug\\abi_compat_diag_on.pdb"
        match pdb_param.last() {
            Some(pdb) => {
                let pdb_path = pdb.to_string_lossy().to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
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
            },
            None => {
                println!("do not fetch program database.");
                return (ProgramDataBase::NonePDBPath, false);
            },
        }
    }
    return (ProgramDataBase::NonePDBPath, false);
}

fn exempt_compile_current_source_by_cache(single_source_file: String, compiler_commands: &mut Vec<std::ffi::OsString>) {

    let command = compiler_commands.clone().into_iter().filter(|arg| 
        !arg.to_string_lossy().contains(&single_source_file));
    
    let temp_commands = command.collect::<Vec<std::ffi::OsString>>();
    *compiler_commands = temp_commands;
}

async fn get_generated_cache_from_storage(key: &str, object_path: &std::path::PathBuf, storage: &std::sync::Arc<dyn crate::cache::cache::Storage>) {
    
    let result = storage.get(key).await;
    match result {
        Ok(cache) => {
            match cache {
                crate::cache::cache::Cache::Hit(value) => {
                    let _ = std::fs::write(object_path, value);
                },
                crate::cache::cache::Cache::Miss => {
                    println!("can't write value to file.");
                },
                _ => {

                },
            }           
        },
        Err(error) => {
            println!("can't get object cache from storage, error code: {:?}.", error);
        },
    }
}

async fn set_generated_cache_to_storage(key: &str, object_path: &std::path::PathBuf, storage: &std::sync::Arc<dyn crate::cache::cache::Storage>) {
    println!("set object cache file: {:?}", object_path);

    let content = std::fs::read(object_path);
    match content {
        Ok(content) => {
            let result = storage.set(key, content).await;
            match result {
                Ok(duration) => {
                    println!("set cache to storage, duration is: {:?}.", duration.as_secs());
                },
                Err(error) => {
                    println!("set local object file to storage filed, error code: {:?}.", error);
                },
            }
        },
        Err(error) => {
            println!("read local object file failed, path: {:?}, error code: {:?}.", object_path, error);
        },
    };
}

async fn exist_source_file_generated(key: &str, storage: &std::sync::Arc<dyn crate::cache::cache::Storage>) -> bool {
    let result = storage.exits(key).await;
    return result;
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