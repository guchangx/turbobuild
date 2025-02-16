
use crew::{compiler::model::{CompilerInput, CompilerOutput, ProcessedResult, ProcessedResults}, replica::project};

pub struct MSVC {
    pub version: String,
}

impl crate::compiler::interface::Compiler for MSVC {

    fn request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<ProcessedResults>) {
        
        let output = request_local_compile_by_preprocessed_source(&compiler_input);
        return output;
    }
}

fn request_local_compile_by_preprocessed_source(compiler_input: &CompilerInput) -> (CompilerOutput, Option<ProcessedResults>) {

    /*
    if !compiler_input.compiler_working_dir.is_empty() {
        let path = std::path::PathBuf::from(&compiler_input.compiler_working_dir);
        if !path.exists() {
            match std::fs::create_dir_all(&path) {
                Ok(_) => {},
                Err(error) => {
                    log::warn!("dist worker create dir {:?} failed. {:?}.", &path, error);
                },
            }
        }
    }
    */

    //replace .cpp/.c to .i
    let commands = compiler_input.compiler_commands.clone();
    
    /* 
    let obj_file:Vec<std::ffi::OsString> = commands.clone().into_iter().filter(|value| value.to_string_lossy().starts_with("/Fo")).collect();
    if !obj_file.is_empty() {
        let mut obj_file_path = obj_file.first().unwrap().to_string_lossy().to_string();
        obj_file_path = obj_file_path.replace("/Fo", "");
        obj_file_path = obj_file_path.replace(r#"\\"#, r"\");
        let path = std::path::PathBuf::from(&compiler_input.compiler_working_dir).join(&obj_file_path);
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
            let path = std::path::PathBuf::from(&compiler_input.compiler_working_dir).join(&path);
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
    */

    let mut next = false;
    let mut combine_commands: Vec<std::ffi::OsString> = commands.windows(2).filter_map(|chunk| {
        if chunk.len() >= 2 {
            if chunk.starts_with(&vec![std::ffi::OsString::from("/I")]) {
                next = true;
                return Some(std::ffi::OsString::from(format!(r#"{} "{}""#, chunk[0].to_string_lossy(), chunk[1].to_string_lossy())));
            }
        }

        if next {
            next = false;
            return None;
        }
        else {
            return Some(chunk[0].clone());
        }
    }).collect();
    
    if commands.len() / 2 != 0 {
        combine_commands.push(commands[commands.len() - 1].clone());
    }

    let (output, results) = request_local_compile(compiler_input.project.clone(), compiler_input.compiler_path.clone(),
                    compiler_input.compiler_working_dir.clone(), combine_commands,
                    compiler_input.build_and_compiler_type.clone(), true);

    return (output, results);
}

fn request_local_compile(project_name: std::ffi::OsString, compiler_path: std::ffi::OsString, compiler_working_dir: std::ffi::OsString, 
                                compiler_commands: Vec<std::ffi::OsString>, build_and_compiler_type: std::ffi::OsString,
                            sync_compile_result: bool) -> (CompilerOutput, Option<ProcessedResults>) {
    use std::io::Read;

    let now = std::time::Instant::now();
    let (status, stdout, stderr) = start_local_compiler(&project_name, &compiler_path, &compiler_working_dir, &compiler_commands);
    let compile_output = String::from_utf8_lossy(&stdout);
    let compile_error = String::from_utf8_lossy(&stderr);
    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    let mut compiled_results: ProcessedResults = Vec::new();
    
    log::trace!("injectd compile status: {}, stdout: {:?} stderr: {:?}", status, compile_output, compile_error);

    if status {

        let lines:Vec<&str> = compile_output.lines().collect();
        
        let (lines, _warning) = filter_compiler_warning_and_message(lines);

        log::debug!("local compile file count: {:?} success, elapsed: {:?}.", lines.len(), now.elapsed());

        let actions = parse_action_from_commands(&project_name, &build_and_compiler_type, &compiler_commands, &compiler_working_dir);

        let generated_object = actions.generated_object;
        let program_database = actions.program_database;

        for line in &lines {
            let line = line.replace(r#"""#, "");
            if line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".i") {
                if sync_compile_result {
                    let mut obj: Option<(std::ffi::OsString, Vec<u8>)> = None;
                    let mut pdb: Option<(std::ffi::OsString, Vec<u8>)> = None;
                    let mut idb: Option<(std::ffi::OsString, Vec<u8>)> = None;
    
                    let mut result = std::path::PathBuf::from("");
                    let object = generated_object.clone();
                    
                    match object {
                        GeneratedObject::PathWithObjName(path) => {
                            result = path;
                        },
                        GeneratedObject::PathWithoutObjName(dir) => {
                            let mut path = dir.join(&line);
                            path.set_extension("obj");
                            result = path;
                        },
                        _ => {
                            log::warn!("fetch result obj file path failed.");
                        }
                    };

                    log::trace!("compile result generated object path: {:?}", result);
                    
                    match std::fs::File::open(&result) {
                        Ok(file) => {
                            let mut contents = Vec::new();
                            let mut file = std::io::BufReader::new(file);
                            let _ = file.read_to_end(&mut contents).unwrap();

                            let origin = repair_original_path(&project_name, &compiler_working_dir, &result);        

                            obj = Some((origin, contents));
                        },
                        Err(error) => {
                            if error.kind() == std::io::ErrorKind::NotFound {
                                log::trace!(".obj file path is not found.");
                            }
                            else {
                                log::trace!(".obj file read failed. {:?}", error);
                            }
                        }
                    };
    
                    result.clear();

                    if lines.last() == Some(&line.as_str()) {
                       
                        match &program_database {
                            ProgramDataBase::PathWithPDBName(path) => {
                                result = path.clone();
                            },
                            ProgramDataBase::PathWithoutPDBName(dir) => {
                                result = dir.join(&line);
                                result.set_extension("pdb");
                            },
                            _ => {
                                log::warn!("fetch result pdb file path failed.");
                            }
                        };

                        log::trace!("compile result program database path {:?}", result);
                        
                        if result.exists() {
                            match std::fs::File::open(&result) {
                                Ok(file) => {
                                    let mut contents = Vec::new();
                                    let mut file = std::io::BufReader::new(file);
                                    let _ = file.read_to_end(&mut contents).unwrap();
                                    let origin = repair_original_path(&project_name, &compiler_working_dir, &result);        
                                    pdb = Some((origin, contents));
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

                            result.set_extension("idb");
                            match std::fs::File::open(&result) {
                                Ok(file) => {
                                    let mut contents = Vec::new();
                                    let mut file = std::io::BufReader::new(file);
                                    let _ = file.read_to_end(&mut contents).unwrap();
                                    let origin = repair_original_path(&project_name, &compiler_working_dir, &result);     
                                    idb = Some((origin, contents));
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
                    }

                    let processed_result = ProcessedResult {
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
                log::trace!("exclude source file, maybe warning and error. {:?}", line);
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

#[derive(Debug, Clone, PartialEq)]
enum GeneratedObject {
    NoneObjPath,
    PathWithObjName(std::path::PathBuf),
    PathWithoutObjName(std::path::PathBuf),
}

fn filter_compiler_warning_and_message(lines: Vec<&str>) -> (Vec<&str>, Vec<&str>) {
    let (files, warning):(Vec<_>, Vec<_>) = lines.into_iter().partition(|item| item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c"));
    return (files, warning);
}

fn exact_compiler_object_file(project: std::borrow::Cow<str>, mut arg: std::borrow::Cow<str>, working_dir: &std::path::PathBuf) -> GeneratedObject {

    let replica = tools::utils::access_replica_dir();
    let replica = std::path::PathBuf::from(replica);

    let split = |pdb: std::path::PathBuf, project: std::borrow::Cow<str>| {
        if project.is_empty() {
            return pdb;
        }
        else {
        
            let components = pdb.components().collect::<Vec<_>>();
            if let Some(index) = components.iter().position(|item| item.as_os_str().to_string_lossy() == project) {
                let result: std::path::PathBuf = components[index + 1..].iter().collect();
                let path = replica.join("Project").join(project.into_owned()).join(result);
                return path;
            }
            else {
                return pdb;
            }            
        }
    };

    let object = arg.to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
    if object.ends_with(".obj") {
        let path = std::path::PathBuf::from(object);
        if path.has_root() {
            let path = split(path, project);
            return GeneratedObject::PathWithObjName(path);
        }
        else {
            let path = replica.join("Project").join(project.into_owned()).join(path);
            return GeneratedObject::PathWithObjName(path);
        }
    }
    else {
        let path = std::path::PathBuf::from(&object);
        if path.has_root() {
            let path = split(path, project);
            return GeneratedObject::PathWithoutObjName(path);
        }
        else {
            let path = working_dir.join(path);
            let path = split(path, project);
            return GeneratedObject::PathWithoutObjName(path);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ProgramDataBase {
    NonePDBPath,
    PathWithPDBName(std::path::PathBuf),
    PathWithoutPDBName(std::path::PathBuf),
}

fn exact_compile_pdb_path(project: std::borrow::Cow<str>, mut arg: std::borrow::Cow<str>, working_dir: &std::path::PathBuf) -> ProgramDataBase {

        
    //"/FdD:\\TrainSpace\\json\\Build\\tests\\abi\\diag\\Debug\\abi_compat_diag_on.pdb"

    let replica = tools::utils::access_replica_dir();
    let replica = std::path::PathBuf::from(replica);

    let split = |pdb: std::path::PathBuf, project: std::borrow::Cow<str>| {
        if project.is_empty() {
            return pdb;
        }
        else {
            let components = pdb.components().collect::<Vec<_>>();
            if let Some(index) = components.iter().position(|item| item.as_os_str().to_string_lossy() == project) {
                let result: std::path::PathBuf = components[index + 1..].iter().collect();
                let path = replica.join("Project").join(project.into_owned()).join(result);
                return path;
            }
            else {
                return pdb;
            }            
        }
    };

    let pdb = arg.to_mut().split_off(3).replace(r#"""#, "").replace(r"\\", r"\");
    let _one = pdb.contains(r"\vc14");
    if pdb.ends_with(".pdb") {
        let path = std::path::PathBuf::from(pdb);
        if path.has_root() {
            let path = split(path, project);
            return ProgramDataBase::PathWithPDBName(path);
        }
        else {
            let path = replica.join("Project").join(project.into_owned()).join(path);
            return ProgramDataBase::PathWithPDBName(path);
        }
    }
    else {
        let path = std::path::PathBuf::from(pdb);
        if path.has_root() {
            let path = split(path, project);
            return ProgramDataBase::PathWithoutPDBName(path);
        }
        else {
            let path = replica.join(path);
            let path = split(path, project);
            return ProgramDataBase::PathWithoutPDBName(path);
        }
    }
}

struct CompileAction {
    pub program_database: ProgramDataBase,
    pub generated_object: GeneratedObject,
}

fn parse_action_from_commands(project_name: &std::ffi::OsString, build_and_compiler_type: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, working_dir: &std::ffi::OsString) -> CompileAction {
    
    let mut pdb = ProgramDataBase::NonePDBPath;
    let mut obj = GeneratedObject::NoneObjPath;
    let working_dir = std::path::PathBuf::from(working_dir);

    if build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || build_and_compiler_type.to_string_lossy().contains("CMake") 
        || build_and_compiler_type.to_string_lossy().contains("Dist") {
        
        let mut default_pdb = false;
        for command in compiler_commands {
            let command = command.to_string_lossy();
            if command.starts_with("/Fd") {
                pdb = exact_compile_pdb_path(project_name.to_string_lossy(), command, &working_dir);
            }
            else if command.starts_with("/Zi") {
                default_pdb = true;
            }
            else if command.starts_with("/Fo") {
                obj = exact_compiler_object_file(project_name.to_string_lossy(), command, &working_dir);
            } 
        }

        if pdb == ProgramDataBase::NonePDBPath && default_pdb {
            let replica = tools::utils::access_replica_dir();
            let replica = std::path::PathBuf::from(replica);
            pdb = ProgramDataBase::PathWithPDBName(replica.join("Project").join(project_name).join("vc140.pdb"));
        }
    }
    else {
        
    }
    return CompileAction { program_database: pdb, generated_object: obj };
}

fn start_local_compiler_with_inject(project: &std::ffi::OsString, compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) 
                        -> (bool, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    
    let line: String = compiler_commands.clone().into_iter()
        .map(|os_string| format!("{} ", os_string.to_string_lossy()))
        .collect();

    let line =  format!(r#""{}" {}"#, compiler_path.to_string_lossy(), line);

    let (status, stdout, stderr) = crate::detours::redirect::msvc_detours(project.clone().into_string().unwrap(), compiler_path.clone().into_string().unwrap(), 
                    line, working_dir.clone().into_string().unwrap());

    return (status, stdout, stderr);
}

fn start_local_compiler(project_name: &std::ffi::OsString, compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) 
                    -> (bool, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {

    log::trace!("project name: {:?}", project_name);
    log::trace!("local compile working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compile content: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    
    let (status, stdout, stderr) = start_local_compiler_with_inject(project_name, compiler_path, working_dir, compiler_commands);
    
    let elapsed = start.elapsed();
    log::info!("compile file with inject elapsed time: {:?}.", elapsed);
    return (status, stdout, stderr);
}


pub fn redirect_stdout_log() {
    use tokio::io::AsyncWriteExt;
    log::info!("redirect stdout log loop thread start.");

    use std::os::windows::ffi::OsStrExt;
    let os_string = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");

    let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
    wchars.push(0);

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().build().unwrap();

    let mut count  = 0;
    unsafe { loop {

        let pipe = winapi::um::namedpipeapi::CreateNamedPipeW(wchars.as_ptr(), winapi::um::winbase::PIPE_ACCESS_INBOUND,  
        winapi::um::winbase::PIPE_TYPE_MESSAGE | winapi::um::winbase::PIPE_READMODE_MESSAGE |  winapi::um::winbase::PIPE_WAIT,
        winapi::um::winbase::PIPE_UNLIMITED_INSTANCES,
        0, 0, 0, std::ptr::null_mut());
        
        if !pipe.is_null() && pipe != winapi::um::handleapi::INVALID_HANDLE_VALUE {
            log::info!("redirect stdout log create new named pipe success. count: {}.", count);
            count += 1;
            if winapi::um::namedpipeapi::ConnectNamedPipe(pipe, std::ptr::null_mut()) == winapi::shared::minwindef::TRUE {
                log::info!("redirect stdout log be connected named pipe.");

                let handle = tools::ptr::HandleBox::new(pipe);
                let _ = runtime.spawn(async move {
                //let _ = std::thread::spawn(move || {

                    log::info!("redirect stdout log read named pipe message task start.");
                    let mut buffer = vec![0u8; 512];
                    let mut bytes: winapi::shared::minwindef::DWORD = 0;
        
                    loop {
                        let result = winapi::um::fileapi::ReadFile(
                            handle.get().to_owned(),
                            buffer.as_mut_ptr() as *mut _,
                            buffer.len() as u32,
                            &mut bytes,
                            std::ptr::null_mut()
                        );
        
                        if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                            let error = winapi::um::errhandlingapi::GetLastError();
                            log::warn!("reaf pipe failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));

                            if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                break;
                            }
                            else {
                                break;
                            }
                        }
                        let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                        //log::info!("redirect: {:?}", output);
                        //println!("redirect: {}", output);
                        tokio::io::stdout().write_all(format!("redirect: {}\n", output).as_bytes()).await.expect("Failed to write to stdout");
                    }
                    winapi::um::namedpipeapi::DisconnectNamedPipe(handle.get().to_owned());
                    winapi::um::handleapi::CloseHandle(handle.get().to_owned());
                    log::warn!("redirect stdout log read named pipe message task exit.");
                });
            }
            else {
                let error = winapi::um::errhandlingapi::GetLastError();
                log::debug!("connect named pipe failed. error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));

                if error == winapi::shared::winerror::ERROR_NO_DATA {
    
                }
                else {
        
                }
                winapi::um::handleapi::CloseHandle(pipe);
            }
        }
        else {
            log::debug!("create named pipe failed.");
        }
    }}
}

fn repair_original_path(project: &std::ffi::OsString, working_dir: &std::ffi::OsString, path: &std::path::PathBuf) -> std::ffi::OsString {

    //E:\\TestFuture\\GammaRay\\GammaRayTool\\build_enable\\common"
    //d:\\turbobuild\\target\\debug\\Replica\\Project\\GammaRayTool\\build_enable\\common\\gammaray_common.dir\\Debug\\lz4.obj"
    let components = path.components().collect::<Vec<_>>();
    
    if let Some(index) = components.iter().position(|item| item.as_os_str().to_string_lossy() == project.to_string_lossy()) {
        let result: std::path::PathBuf = components[index + 1..].iter().collect();
        
        let base = std::path::PathBuf::from(working_dir);

        let base = base.components().collect::<Vec<_>>();
        if let Some(index) = base.iter().position(|item| item.as_os_str().to_string_lossy() == project.to_string_lossy()) {
            let base = base[..index + 1].iter().collect::<std::path::PathBuf>();
            
            let path = base.join(result);
            return path.as_os_str().to_owned();
        }
        else {
            let path = path.join(result);
            return path.as_os_str().to_owned();
        }
    }
    else {
        return path.as_os_str().to_owned();
    }            
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn compile_sourcefile_inject_test() {
        println!("run msvc .cpp file test inject");
        tools::logger::init_once_logger();
        
        std::thread::spawn(||{
            redirect_stdout_log();
        });

        //cargo test --package cocrew --lib -- compiler::msvc::tests::compile_sourcefile_inject_test --exact --show-output
        
        let win_compile_env = crew::platform::windows::WindowsCompilerEnv::default();
        let mut compiler_path = std::path::PathBuf::from(win_compile_env.compiler_path);
        compiler_path = compiler_path.join("Hostx64/x64/cl.exe");

        let mut working_dir = std::ffi::OsString::from("");
        let dir = std::env::current_dir().unwrap();
        let dir = dir.to_string_lossy();
        let index = dir.find("turbobuild");
        if let Some(index) = index {
            let path = &dir[0..index];
            let mut path = std::path::PathBuf::from(path);
            path.push("turbobuild");
            path.push("draft");

            working_dir = path.into_os_string();
        }
        
        println!("draft dir: {}", working_dir.to_string_lossy());

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();

        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/EHs"));
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

        for sdk_include in win_compile_env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {:#?}"#, sdk_include)));
        }
        
        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {:#?}"#, win_compile_env.msvc_includes_path)));
        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));

        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, working_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, working_dir.to_string_lossy())));

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), 
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands);
        assert!(status);
        println!("compile stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile stderr: {}", String::from_utf8_lossy(&stderr));
    }

    #[test]
    fn compile_preprocessedfile_with_inject() {
        println!("run msvc .i file test inject");
        tools::logger::init_once_logger();
        
        std::thread::spawn(||{
            redirect_stdout_log();
        });

        //cargo test --package cocrew --lib -- compiler::msvc::tests::compile_preprocessedfile_with_inject --exact --show-output
        
        let win_compile_env = crew::platform::windows::WindowsCompilerEnv::default();
        let mut compiler_path = std::path::PathBuf::from(win_compile_env.compiler_path);
        compiler_path = compiler_path.join("Hostx64/x64/cl.exe");

        let mut working_dir = std::ffi::OsString::from("");
        let dir = std::env::current_dir().unwrap();
        let dir = dir.to_string_lossy();
        let index = dir.find("turbobuild");
        if let Some(index) = index {
            let path = &dir[0..index];
            let mut path = std::path::PathBuf::from(path);
            path.push("turbobuild");
            path.push("draft");

            working_dir = path.into_os_string();
        }
        
        println!("draft dir: {}", working_dir.to_string_lossy());

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();

        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/EHs"));
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

        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.i"#, working_dir.to_string_lossy())));

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), 
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands);
        assert!(status);
        println!("compile .i file stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile .i file stderr: {}", String::from_utf8_lossy(&stderr));
    }
    
    unsafe fn named_pipe_send_message_test() {
        use std::os::windows::ffi::OsStrExt;
        let id = std::thread::current().id();
        println!("run named pipe send message test, thread {:?}", id);

        let name = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");
        let name = name.encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

  
                if winapi::um::namedpipeapi::WaitNamedPipeW(name.as_ptr(), 500) == winapi::shared::minwindef::TRUE {
                   println!("wait named pipe success");
                }
                else {
                   println!("wait named pipe failed");
                }

    
        if true {
            let pipe = winapi::um::fileapi::CreateFileW(name.as_ptr(), 
                winapi::um::winnt::GENERIC_WRITE, 
                0,
                std::ptr::null_mut(), 
                winapi::um::fileapi::OPEN_EXISTING, 
                winapi::um::winnt::FILE_ATTRIBUTE_NORMAL, 
                winapi::shared::ntdef::NULL
            );
     
            if !pipe.is_null() && pipe != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let handle = tools::ptr::HandleBox::new(pipe);

                for i in 0..20 {
                    let message = String::from(format!("test pipe {} thread: {:?} ...", i, id));
                    let mut bytes: winapi::shared::minwindef::DWORD = 0;
                    let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
                    let result = winapi::um::fileapi::WriteFile(
                        handle.get().to_owned(),
                        message.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
                        message.len() as u32,
                        &mut bytes,
                        &mut overlapped
                    );
    
                    if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                        let error = winapi::um::errhandlingapi::GetLastError();
                        if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                            break;
                        }
                        println!("write pipe error, failed code: {} message: {}", error, tools::utils::get_winapi_error_message(error));
                        break;
                    }
                };

                winapi::um::fileapi::FlushFileBuffers(handle.get().to_owned());
                winapi::um::handleapi::CloseHandle(handle.get().to_owned()); 
            }
            else {
                let error = winapi::um::errhandlingapi::GetLastError();
                println!("create pipe failed, error code: {}, message: {}", error, tools::utils::get_winapi_error_message(error));
            }
        }
    }

    //cargo test --package cocrew --lib -- compiler::msvc::tests::named_pipe_receive_message_test --exact --show-output --nocapture
    #[test]
    fn named_pipe_receive_message_test() {
        println!("run named_pipe_receive_message_test start.");
        tools::logger::init_once_logger();

        let _handle = std::thread::spawn(||{
            redirect_stdout_log();
        });

        std::thread::sleep(std::time::Duration::from_millis(1000));
        
        let mut handles = Vec::new();
        for _i in 0..10 {
            
            std::thread::sleep(std::time::Duration::from_millis(100)); //root issue.
            let handle = std::thread::spawn(||{
                unsafe {
                    named_pipe_send_message_test();
                }
            });
            //handle.join().unwrap();
            //std::thread::sleep(std::time::Duration::from_millis(1000));
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();    
        }
        println!("run named_pipe_receive_message_test send message done.");

        //_handle.join().unwrap();
    }
}