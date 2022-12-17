extern crate regex;
pub struct MSVC;
use std::{ops::Index};

#[async_trait]
impl crate::compiler::compiler::Compiler for MSVC {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                                compile_input: super::compiler::CompileInput, _pool: &tokio::runtime::Handle)
                                -> super::compiler::CompileOutput {
        let output = request_dist_compile(&working_parameters, &compile_input);
        //let output = request_msvc_compile(working_parameters, compile_input, pool).await;
        return output;
    }

    async fn dist_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
            compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle)
                                    -> super::compiler::CompileOutput {
        
        let mut compiler_env = crate::platform::windows::WindowsCompilerEnv::default();

        if compile_input.preprocessed_source.is_none() {
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

            let output = request_msvc_compile(params, compile_input, pool).await;
            return output;
        }
        else {
            let output = request_local_preprocessed_compile(&compile_input, pool);
            return output;
        }


    }
}

async fn request_msvc_compile(working_parameters: crate::buildturbo::WorkingParameters, msvc_compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle) 
                                        -> super::compiler::CompileOutput {
    let storage = working_parameters.storage;
    let env = working_parameters.compiler_env;

    let (mut compiler_commands, compiler_path) = parse_compiler_input_command(msvc_compile_input.clone(), &env);

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
    if false {
        for (file, path) in source_file {
        
            let key = crate::utils::hasher::Digest::file(path.clone(), pool).await;
            
            match key {
                Ok(key) => {
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
    
    println!("determine whether need compile");

    let exists_source_file_in_command = determine_whether_need_compile(compiler_commands.clone());
    
    if exists_source_file_in_command {
        let mut output = super::compiler::CompileOutput::default();
        let value = crate::utils::grade::calculate_machine_residual_performance();
        if value < 85 {
            output = request_local_compile(compiler_path, msvc_compile_input.compiler_working_dir, compiler_commands.clone());
        }
        else {
            if true {
                // dist with preprocessed source
                request_local_precompile(&working_parameters.network_client, &compiler_path, &msvc_compile_input.compiler_working_dir, &compiler_commands.clone());
            }
            else {
                //dist with source file and include file
                let parameters =  crate::buildturbo::WorkingParameters {
                    storage: storage.clone(),
                    dist: working_parameters.dist,
                    compiler_env: env,
                    network_client: working_parameters.network_client,
                };

                output = request_dist_compile(&parameters, &msvc_compile_input);
            }
        }

        if output.compile_status {
            println!("compiled filename {:?} need_compile_file_key: {:?}", output.compiled_filename.clone(), need_compile_file_key.clone());
            for compiled_file in output.compiled_filename.clone() {
                let compiled_file = compiled_file.to_str().unwrap();
                let key = need_compile_file_key.get(compiled_file);
                match key {
                    Some(key) => {

                        match &object {
                            GeneratedObject::PathWithObjName(object_filepath_with_filename) => {
                                set_generated_cache_to_storage(&key, object_filepath_with_filename, &storage).await;
                            },
                            GeneratedObject::PathWithoutObjName(object_filepath_without_filename) => {
                                let suffix_index = compiled_file.rfind('.').unwrap();
                                let mut filename = compiled_file.to_owned();
                                let _ = filename.split_off(suffix_index);
                                filename.push_str(".obj");
                                let object_path = object_filepath_without_filename.join(filename);
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
        return output;
    }
    else {
        let compiled_filename: Vec<std::ffi::OsString> = Vec::new();
        let output = super::compiler::CompileOutput {
            compiled_filename,
            compile_status: false,
            compile_output: std::ffi::OsString::from("don't need compile anything."),
            compiled_result: None,
        };
        return output;
    }
}

fn request_local_precompile(network: &crate::network::client::NetworkClient, compiler_path: &std::ffi::OsString, compiler_working_dir: &std::ffi::OsString, 
    compiler_commands: &Vec<std::ffi::OsString>) -> super::compiler::CompileOutput {

    //add precompile args into command
    let mut commands = compiler_commands.to_owned();
    commands.insert(0, std::ffi::OsString::from(r#"/EP"#));
    let (compile_status, compile_output) = start_local_compiler(compiler_path, compiler_working_dir, &commands);
    if compile_status {
        println!("preprocessed source file {:?}", compile_output);

        let msvc_compile_input = super::compiler::CompileInput {
            compiler_path_or_arch: compiler_path.to_owned(),
            compiler_working_dir: compiler_working_dir.to_owned(),
            compiler_commands: compiler_commands.to_owned(),
            build_and_compiler_type: std::ffi::OsString::from("MSVC Precompile"),
            preprocessed_source: Some(std::ffi::OsString::from(compile_output)),
            env_input: None
        };
        request_dist_compile_with_preprocessed_source(network, &msvc_compile_input);
    }
    else {
        println!("preprocessed source file failed");
    }

    return super::compiler::CompileOutput::default();
}

fn request_local_compile(compiler_path: std::ffi::OsString, compiler_working_dir: std::ffi::OsString, 
                                compiler_commands: Vec<std::ffi::OsString>) -> super::compiler::CompileOutput {

    let (compile_status, compile_output) = start_local_compiler(&compiler_path, &compiler_working_dir, &compiler_commands);

    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    if compile_status {
        let output = compile_output.lines();
        let mut compiled_result: Vec<(std::ffi::OsString, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
        for line in output {
            let line = line.replace(r#"""#, "");
            if line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".i") {
                

                let mut obj: Vec<u8> = Vec::new();
                let mut pdb: Vec<u8> = Vec::new();
                let mut idb: Vec<u8> = Vec::new(); 

                let working_path = std::path::PathBuf::from(compiler_working_dir.to_owned());
                let object = fetch_compiler_object_file(std::ffi::OsString::from(""), compiler_commands.to_owned(), working_path);
                match object {
                    GeneratedObject::PathWithObjName(dir) => {
                        let mut path = dir.join(&line);
                        path.set_extension("obj");

                        match std::fs::read(&path) {
                            Ok(contents) => {
                                obj = contents;
                            },
                            Err(error) => {
                                if error.kind() == std::io::ErrorKind::NotFound {
                                    println!("obj file path is not found.");
                                }
                                else {
                                    println!("obj file read failed. {:?}", error);
                                }
                            }
                        }
                        
                        path.set_extension("pdb");

                        match std::fs::read(&path) {
                            Ok(contents) => {
                                pdb = contents;
                            },
                            Err(error) => {
                                if error.kind() == std::io::ErrorKind::NotFound {
                                    println!(".pdb file path is not found.");
                                }
                                else {
                                    println!(".pdb file read failed. {:?}", error);
                                }
                            }
                        }

                        path.set_extension("idb");

                        match std::fs::read(&path) {
                            Ok(contents) => {
                                idb = contents;
                            },
                            Err(error) => {
                                if error.kind() == std::io::ErrorKind::NotFound {
                                    println!(".idb file path is not found.");
                                }
                                else {
                                    println!(".idb file read failed. {:?}", error);
                                }
                            }
                        }
                        compiled_result.push((std::ffi::OsString::from(&line), obj, pdb, idb));
                    },
                    GeneratedObject::PathWithoutObjName(_path) => {

                    },
                    _ => {

                    }
                }
                compiled_filename.push(std::ffi::OsString::from(line));
            }
        }
    };
    
    let result = super::compiler::CompileOutput {
        compiled_filename,
        compile_status: compile_status,
        compile_output: std::ffi::OsString::from(compile_output),
        compiled_result: None,
    };

    return result;
}

fn request_local_preprocessed_compile(msvc_compile_input: &super::compiler::CompileInput, _pool: &tokio::runtime::Handle) -> super::compiler::CompileOutput{

    let output = request_local_compile(msvc_compile_input.compiler_path_or_arch.clone(),
                    msvc_compile_input.compiler_working_dir.clone(), msvc_compile_input.compiler_commands.clone());

    return output;
}

fn request_dist_compile(working_parameters: &crate::buildturbo::WorkingParameters, msvc_compile_input: &super::compiler::CompileInput) -> super::compiler::CompileOutput {

    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
    let local_compiler_arch = msvc_compile_input.compiler_path_or_arch.to_str().unwrap();
    let compiler_dir = std::path::Path::new(&working_parameters.compiler_env.compiler_path).join("Hostx64").join(local_compiler_arch);

    println!("vs compiler install dir: {:?}", compiler_dir);

    let winsdk_path = working_parameters.compiler_env.winkits_includes_path.first().unwrap();
    
    let sender = crate::syncfile::sender::Sender::new(&working_parameters.network_client);

    let mut dist_msvc_compiler_path: std::ffi::OsString;
    let mut dist_msvc_include_path: std::ffi::OsString;
    let mut win_kits_include_dir: std::ffi::OsString;

    (dist_msvc_compiler_path, dist_msvc_include_path, win_kits_include_dir)
        = sender.dist_kits_and_tool_pre_sync(winsdk_path, compiler_dir.to_str().unwrap());

    if dist_msvc_compiler_path.is_empty() {
        println!("msvc toolchain sync");
        let path = std::path::PathBuf::from(compiler_dir.to_str().unwrap());
        if path.is_dir() {
            (dist_msvc_compiler_path, dist_msvc_include_path) = sender.sync_toolchain(&path);
        }
    }
    else {
        println!("tool chain sync have done");
    }

    if win_kits_include_dir.is_empty() {
        println!("windows kits sync");
        let mut path = std::path::PathBuf::from(winsdk_path);
        if path.is_dir() && path.pop() {
            win_kits_include_dir = sender.sync_windows_kits(&path);
        }
    }
    else {
        println!("windows kits sync have done");
    }

    if !dist_msvc_compiler_path.is_empty() {
        let mut path = std::path::PathBuf::from(dist_msvc_compiler_path);
        if !path.ends_with("cl.exe") {
            path.set_file_name("cl.exe")
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

    println!("input env: {:?}", env_input.clone());
    
    input.build_and_compiler_type = std::ffi::OsString::from("Dist MSVC");

    let response = sender.dist_compile(&input);
    
    return response;
}

fn request_dist_compile_with_preprocessed_source(network: &crate::network::client::NetworkClient, msvc_compile_input: &super::compiler::CompileInput) 
                                        -> super::compiler::CompileOutput {
    
    let sender = crate::syncfile::sender::Sender::new(network);
    let mut dist_msvc_compiler_path= sender.dist_kits_and_tool_pre_sync("", msvc_compile_input.compiler_path_or_arch.to_str().unwrap()).0;
    if dist_msvc_compiler_path.is_empty() {
        println!("sync msvc bin for preprocessed source.");

        let mut path = std::path::PathBuf::from(msvc_compile_input.compiler_path_or_arch.to_str().unwrap());

        if path.is_file() {
            path.pop();
        }

        if path.is_dir() {
            (dist_msvc_compiler_path, _) = sender.sync_toolchain(&path);
        }
   
        let mut input = msvc_compile_input.to_owned();
        input.build_and_compiler_type = std::ffi::OsString::from("Dist Precompile");
        input.compiler_path_or_arch = dist_msvc_compiler_path;
    
        let response = sender.dist_compile(&input);
        return response;
    }
    else {
        return super::compiler::CompileOutput::default(); 
    }
}

fn start_local_compiler(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> (bool, String) {
    use std::process::Stdio;

    let now_start = chrono::Local::now();
    println!("start time: {:?}, compiler path: {:?}, start content: {:?}", now_start.format("%Y-%m-%d %H:%M:%S%.3f").to_string(), compiler_path, compiler_commands);

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
                        let output_context = String::from_utf8_lossy(&output.stdout);
                        let now_end = chrono::Local::now();
                        //println!("file: {:#?}", output_context);
                        println!("end time: {:?}, Id: {:?}", now_end.format("%Y-%m-%d %H:%M:%S%.3f").to_string(), child_id);
                        return (true, output_context.into_owned());
                    }
                    else {
                         let output_context = String::from_utf8_lossy(&output.stdout);
                         println!("build error: {:?}", output_context);
                         return (false, output_context.into_owned());
                    }
                },
                Err(error) => {
                    println!("spawn compile child process error: {:?}", error);
                    let mut error_description = String::from("compile child wait output error: ");
                    error_description.push_str(error.to_string().as_str());
                    return (false,  error_description);
                },
            }
        },
        Err(error) => {
            println!("spawn compile child process error: {:?}", error);
            let mut error_description = String::from("spawn compile child process error: ");
            error_description.push_str(error.to_string().as_str());
            return (false,  error_description);
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
                
                println!("capture {:?}", capture);

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
    let args_vec = compiler_commands.split(" ").map(|arg| String::from(arg)).collect::<Vec<_>>();
    let mut args: Vec<std::ffi::OsString> = Vec::new();

    for arg in args_vec {
        if arg.starts_with("/Fo") || arg.starts_with("/Fd") {
            let arg_without_enclosed_quotation = arg.replace('"', "");
            args.push(std::ffi::OsString::from(arg_without_enclosed_quotation));
        }
        else {
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
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSVC") {
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
        println!("compiler_path: {:?}", compiler_path);

        args = extract_macro_contain_space_arg(&mut commands);

        let mut args_with_path = extract_path_ecnclosed_quotation_arg(&mut commands);
        println!("ecnclosed quotation: {:?}", args_with_path);
        args.append(&mut args_with_path);

        let mut args_others = split_commonds_by_space(&mut commands);
        args.append(&mut args_others);

        if args.is_empty() {
            println!("compiler don't effective extract commands.")
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
    else if  compile_input.build_and_compiler_type.to_string_lossy().contains("Cmake") {
        compiler_path = compile_input.compiler_path_or_arch;
        return (args, compiler_path);
    }
    else {
        return (args, compiler_path);
    }
}

fn fetch_compiler_source_file(build_and_compiler_type: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>, working_dir: std::path::PathBuf) 
                                    -> Option<std::collections::HashMap<String, std::path::PathBuf>> {

    if build_and_compiler_type.to_string_lossy().contains("MSBuild") || 
            build_and_compiler_type.to_string_lossy().contains("CMake") || 
            build_and_compiler_type.to_string_lossy().contains("Dist") {
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

    if build_and_compiler_type.to_string_lossy().contains("MSBuild") || build_and_compiler_type.to_string_lossy().contains("CMake") {

        let object_param = compiler_commands.into_iter().filter(|arg| arg.to_string_lossy().starts_with("/Fo")).collect::<Vec<_>>();

        match object_param.last() {
            Some(object) => {
                let object_path = object.to_string_lossy().to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
                if object_path.ends_with(".obj") {
                    let path = working_dir.join(object_path);
                    return GeneratedObject::PathWithObjName(path);
                }
                else {
                    let path = working_dir.join(object_path);
                    return GeneratedObject::PathWithoutObjName(path);
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