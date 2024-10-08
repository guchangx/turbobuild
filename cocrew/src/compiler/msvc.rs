use crew::compiler::model::{CompilerInput, CompilerOutput, ProcessedResults};
pub struct MSVC {
    pub version: String,
}

impl crate::compiler::interface::Compiler for MSVC {

    fn dist_request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<ProcessedResults>) {
        
    }
    
}

fn request_local_compile_by_preprocessed_source(compiler_input: &CompilerInput, _pool: std::sync::Arc<tokio::runtime::Handle>) -> (CompilerOutput, Option<ProcessedResults>) {

    //replace .cpp/.c to .i
    let commands = compiler_input.compiler_commands.clone();

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

fn request_local_compile(compiler_path: std::ffi::OsString, compiler_working_dir: std::ffi::OsString, 
                                compiler_commands: Vec<std::ffi::OsString>, build_and_compiler_type: std::ffi::OsString,
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
        compiled_filename,
        compile_status: status,
        compile_output: std::ffi::OsString::from(compile_output.to_string()),
    };

    return (result, Some(compiled_results));
}

fn request_local_compile(compiler_path: std::ffi::OsString, compiler_working_dir: std::ffi::OsString, 
                        compiler_commands: Vec<std::ffi::OsString>, build_and_compiler_type: std::ffi::OsString,
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
    compiled_filename,
    compile_status: status,
    compile_output: std::ffi::OsString::from(compile_output.to_string()),
};

return (result, Some(compiled_results));
}

fn start_local_compiler_with_inject(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) {
    
    let command_line: String = compiler_commands.clone().into_iter()
    .map(|os_string| format!("{} ", os_string.into_string().unwrap()))
    .collect();

    let (status, readbuffer, errorbuffer) = crate::detours::redirect::msvc_detours(
        compiler_path.clone().into_string().unwrap(), 
        command_line, 
        working_dir.clone().into_string().unwrap()
    );
    if status == true {
        log::trace!("detours success");
    }
    else {
        log::warn!("detours failed, {:?}", errorbuffer);
    }
}

fn start_local_compiler(compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>) -> (bool, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    use std::process::Stdio;

    log::trace!("local compile working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compile content: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    
    start_local_compiler_with_inject(compiler_path, working_dir, compiler_commands);
    return (false, std::sync::Arc::new(vec![]), std::sync::Arc::new(vec![]));

    let elapsed = start.elapsed();
    log::info!("compile file elapsed time: {:?}.", elapsed);
}