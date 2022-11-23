extern crate regex;

pub struct MSVC;

use std::{ops::Index};


#[async_trait]
impl crate::compiler::compiler::Compiler for MSVC {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                                compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle)
                                -> super::compiler::CompileOutput {
        let output = request_dist_compile(&working_parameters, &compile_input);
        //let output = request_msvc_compile(working_parameters, compile_input, pool).await;
        return output;
    }

    async fn dist_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
        compiler_env: crate::platform::windows::WindowsCompilerEnv, compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle)
                                    -> super::compiler::CompileOutput {
        let params = crate::buildturbo::WorkingParameters {
                storage: working_parameters.storage,
                dist: working_parameters.dist,
                compiler_env: compiler_env,
                network_client: working_parameters.network_client,
        };
        let output = request_msvc_compile(params, compile_input, pool).await;
        return output;
    }
}

async fn request_msvc_compile(working_parameters: crate::buildturbo::WorkingParameters, msvc_compile_input: super::compiler::CompileInput, pool: &tokio::runtime::Handle) 
                                        -> super::compiler::CompileOutput {
    let storage = working_parameters.storage;
    let env = working_parameters.compiler_env;

    let (mut compiler_commands, compiler_path) = parse_compiler_input_command(msvc_compile_input.clone(), &env);

    let working_path = std::path::PathBuf::from(msvc_compile_input.compiler_working_dir.to_string_lossy().to_string());
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

    let exists_source_file_in_command = determine_whether_need_compile(compiler_commands.clone());
    
    if exists_source_file_in_command {

        let value = crate::utils::grade::calculate_machine_residual_performance();
        if value > 85 {

            let output = request_local_compile(compiler_path, msvc_compile_input.compiler_working_dir, compiler_commands.clone());
            
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
            let parameters =  crate::buildturbo::WorkingParameters {
                    storage: storage,
                    dist: working_parameters.dist,
                    compiler_env: env,
                    network_client: working_parameters.network_client,
            };
            let output = request_dist_compile(&parameters, &msvc_compile_input);
            return output;
        }
    }
    else {
        let compiled_filename: Vec<std::ffi::OsString> = Vec::new();
        let output = super::compiler::CompileOutput {
            compiled_filename,
            compile_status: false,
            compile_output: std::ffi::OsString::from("don't need compile anything."),
        };
        return output;
    }
}

fn request_local_compile(compiler_path: std::ffi::OsString, compiler_working_dir: std::ffi::OsString, 
                                compiler_commands: Vec<std::ffi::OsString>) -> super::compiler::CompileOutput {

    let (compile_status, compile_output) = start_local_compiler(compiler_path, compiler_working_dir, compiler_commands);

    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    if compile_status {
        let output = compile_output.lines();
        
        for line in output {
            let line = line.replace(r#"""#, "");
            if line.ends_with(".cpp") || line.ends_with(".c") {
                compiled_filename.push(std::ffi::OsString::from(line));
            }
        }
    };

    let result = super::compiler::CompileOutput {
        compiled_filename,
        compile_status: compile_status,
        compile_output: std::ffi::OsString::from(compile_output),
    };

    return result;
}

fn request_dist_compile(working_parameters: &crate::buildturbo::WorkingParameters, msvc_compile_input: &super::compiler::CompileInput) -> super::compiler::CompileOutput {

    let local_compiler = msvc_compile_input.compiler_path.to_str().unwrap();
    let winsdk_path = working_parameters.compiler_env.winsdk_includes_path.first().unwrap();
    
    let sender = crate::syncfile::sender::Sender::new(&working_parameters.network_client);
    let pre_sync_reponse = working_parameters.network_client.dist_kits_and_tool_pre_sync(winsdk_path, local_compiler);
    
    if pre_sync_reponse.toolchain_path.is_empty() {
        sender.sync_tool_chain(local_compiler);
    }
    else if pre_sync_reponse.windows_kits_path.is_empty() {
        sender.sync_tool_chain(winsdk_path);
    }
    else {
        
    }

    let env = crate::platform::windows::WindowsCompilerEnv {
            winsdk_includes_path: vec![pre_sync_reponse.windows_kits_path.to_string_lossy().to_string()],
            compiler_path: std::path::PathBuf::from(pre_sync_reponse.toolchain_path.clone()),
            msvc_includes_path: std::path::PathBuf::from(pre_sync_reponse.toolchain_path),
            msvc_version: String::from(""),
            env_args: String::from(""),
    };

    sender.dist_compile(&env, msvc_compile_input);

    let compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    let result = super::compiler::CompileOutput {
        compiled_filename,
        compile_status: true,
        compile_output: std::ffi::OsString::from(""),
    };

    return result;
}

fn start_local_compiler(compiler_path: std::ffi::OsString, working_dir: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>) -> (bool, String) {
    use std::process::Stdio;

    let now_start = chrono::Local::now();
    println!("start time: {:?}, copiler path: {:?}, start content: {:?}", now_start.format("%Y-%m-%d %H:%M:%S%.3f").to_string(), compiler_path, compiler_commands);

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
                println!("else capture {:?}", capture);
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

        args = extract_macro_contain_space_arg(&mut commands);

        let mut args_with_path = extract_path_ecnclosed_quotation_arg(&mut commands);
        println!("ecnclosed quotation: {:?}", args_with_path);
        args.append(&mut args_with_path);

        let mut args_others = split_commonds_by_space(&mut commands);
        args.append(&mut args_others);

        if args.is_empty() {
            println!("compiler don't effective extract commands.")
        }
        let winkits_includes = &working_compiler_env.winsdk_includes_path;
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
        compiler_path = compile_input.compiler_path;
        return (args, compiler_path);
    }
    else {
        return (args, compiler_path);
    }
}

fn fetch_compiler_source_file(build_and_compiler_type: std::ffi::OsString, compiler_commands: Vec<std::ffi::OsString>, working_dir: std::path::PathBuf) 
                                    -> Option<std::collections::HashMap<String, std::path::PathBuf>> {

    if build_and_compiler_type.to_string_lossy().contains("MSBuild") || 
            build_and_compiler_type.to_string_lossy().contains("CMake") {
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