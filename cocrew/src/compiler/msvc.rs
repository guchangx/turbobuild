
use crew::compiler::model::{CompilerInput, CompilerOutput, ProcessedResult, ProcessedResults};

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

    //replace .cpp/.c to .i
    let commands = compiler_input.compiler_commands.clone();

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
    use std::ops::Index;
    
    let now = std::time::Instant::now();
    let (status, stdout, stderr) = start_local_compiler(&project_name, &compiler_path, &compiler_working_dir, &compiler_commands);
    let compile_output = String::from_utf8_lossy(&stdout);
    let compile_error = String::from_utf8_lossy(&stderr);
    let mut compiled_filename: Vec<std::ffi::OsString> = Vec::new();
    let mut compiled_results: ProcessedResults = Vec::new();
    
    log::trace!("injectd compile status: {}, stderr: {} stdout: {}", status, compile_output, compile_error);

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

    use std::os::windows::ffi::OsStrExt;
    let os_string = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");

    let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
    wchars.push(0);

    unsafe {
        let pipe = winapi::um::namedpipeapi::CreateNamedPipeW(wchars.as_ptr(), winapi::um::winbase::PIPE_ACCESS_DUPLEX,  
        winapi::um::winbase::PIPE_TYPE_MESSAGE | winapi::um::winbase::PIPE_READMODE_MESSAGE |  winapi::um::winbase::PIPE_WAIT, winapi::um::winbase::PIPE_UNLIMITED_INSTANCES,
        0, 0, 0, std::ptr::null_mut());

        if !pipe.is_null() {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
            if winapi::um::namedpipeapi::ConnectNamedPipe(pipe, &mut overlapped) == winapi::shared::minwindef::TRUE {
                let mut buffer = vec![0u8; 512];
                let mut bytes: winapi::shared::minwindef::DWORD = 0;
                let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
    
                loop {
                    let result = winapi::um::fileapi::ReadFile(
                        pipe,
                        buffer.as_mut_ptr() as *mut _,
                        buffer.len() as u32,
                        &mut bytes,
                        &mut overlapped
                    );
    
                    if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                        let error = winapi::um::errhandlingapi::GetLastError();
                        if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                            break;
                        }
                        println!("reaf pipe failed, error code: {}", error);
                        break;
                    } 
                    let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                    //log::info!("redirect: {:?}", output);
                    println!("redirect: {}", output);
                }
            }
            else {
                log::debug!("connect named pipe failed.");
            }
            winapi::um::namedpipeapi::DisconnectNamedPipe(pipe);
            winapi::um::handleapi::CloseHandle(pipe);            
        }
        else {
            log::debug!("create named pipe failed.");
        }
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

        //cargo test --package cocrew --lib -- compiler::msvc::tests::test_inject --exact --show-output
        
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

        //cargo test --package cocrew --lib -- compiler::msvc::tests::test_inject --exact --show-output
        
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

        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.i"#, working_dir.to_string_lossy())));

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), 
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands);
        assert!(status);
        println!("compile .i file stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile .i file stderr: {}", String::from_utf8_lossy(&stderr));
    }
}