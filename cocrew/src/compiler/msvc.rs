
use crew::{communicate::package::CompileRecv, compiler::model::{CompiledResult, CompiledResults, CompilerInput, CompilerOutput}, replica::project};
use std::io::Read;

pub struct MSVC {
    pub version: String,
    pub out_err_stream: crate::compiler::msvc::CompiledResultsStream,
}

pub struct OutAndErrStream {
    pub stdout: std::sync::mpsc::Sender<Vec<u8>>,
    pub stderr: std::sync::mpsc::Sender<Vec<u8>>,
}

pub struct CompiledResultsStream {
    pub stdout: std::sync::mpsc::Sender<crew::compiler::model::CompiledResults>,
    pub stderr: std::sync::mpsc::Sender<crew::compiler::model::CompiledResults>,
}

impl crate::compiler::interface::Compiler for MSVC {

    fn request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<CompiledResults>) {
        let output = request_local_compile_by_preprocessed_source(&compiler_input, &self.out_err_stream);
        return output;
    }
}

fn request_local_compile_by_preprocessed_source(compiler_input: &CompilerInput, out_err_stream: &crate::compiler::msvc::CompiledResultsStream) -> (CompilerOutput, Option<CompiledResults>) {

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

    //replace .cpp/.c/.cc to .i
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

    log::debug!("origin working dir: {:?}", compiler_input.compiler_working_dir);
    let replica_working_dir = redirect_working_dir(&compiler_input.compiler_working_dir, &compiler_input.solution);

    log::debug!("orgin compiler: {:?}", compiler_input.compiler_path);
    let replica_compiler = redirect_compiler_path(compiler_input.compiler_path.clone());

    if replica_compiler.is_some() {
        let mut input = CompilerInput::default();
        input.solution = compiler_input.solution.clone();
        input.project = compiler_input.project.clone();
        input.compiler_path = replica_compiler.unwrap();
        input.compiler_working_dir = replica_working_dir.unwrap();
        input.compiler_commands = combine_commands;
        input.build_and_compiler_type = compiler_input.build_and_compiler_type.clone();

        let (output, results) = request_local_compile(&input, compiler_input.compiler_working_dir.clone(), out_err_stream);
        return (output, results);
    }
    else {
        let mut output = CompilerOutput::default();
        output.status = 1;
        output.err = std::sync::Arc::new("can not find compiler in replica dir.".as_bytes().to_vec());
        return (output, None);  
    }
}

fn request_local_compile(compiler_input: &CompilerInput, origin_working_dir: std::ffi::OsString,
                            out_err_stream: &crate::compiler::msvc::CompiledResultsStream)
                            -> (CompilerOutput, Option<CompiledResults>) {

    let now = std::time::Instant::now();
    let (out_sender, out_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    let (err_sender, err_receiver) = std::sync::mpsc::channel::<Vec<u8>>();

    let stdout_err_stream = OutAndErrStream {
        stdout: out_sender,
        stderr: err_sender,
    };
    
    let solution_name = compiler_input.solution.clone();
    let project_name = compiler_input.project.clone();

    let compiler_path = compiler_input.compiler_path.clone();
    let replica_working_dir = compiler_input.compiler_working_dir.clone();
    let compiler_commands = compiler_input.compiler_commands.clone();

    let actions = parse_action_from_commands(&compiler_input);
    let generated_object = actions.generated_object;
    let program_database = actions.program_database;

    let out_stream = out_err_stream.stdout.clone();

    let solution_name_ = solution_name.clone();
    let origin_working_dir_ = origin_working_dir.clone();
    let generated_object_ = generated_object.clone();

    let stream_objfiles = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let unready_objfiles = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let unready_objfiles_ = unready_objfiles.clone();
    let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {

        while let Ok(data) = out_receiver.recv() {

            stream_objfiles.lock().unwrap().push(data.clone());

            let line = String::from_utf8_lossy(&data);
            log::info!("stream stdout: {:?}", line);

            line.lines().for_each(|item| {
                if !item.is_empty() {
                    let item = std::borrow::Cow::from(item);
                    let objfile = pre_return_local_compile_result_objfiles(&item, generated_object_.clone(), &solution_name_, &origin_working_dir_, &out_stream);
                    if let Some(objfile) = objfile {
                        unready_objfiles_.lock().unwrap().insert(objfile.0, objfile.1);
                    }
                }
            });
        }

        drop(out_stream);
        log::info!("stream stdout end");
    });

    let err_stream = out_err_stream.stderr.clone();

    let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {
        while let Ok(data) = err_receiver.recv() {
            log::info!("stream stderr: {:?}", String::from_utf8_lossy(&data));

            //let _ = err_stream.send(data);
        }
        log::info!("stream stderr end");
        drop(err_stream);
    });

    let (status, stdout, stderr) = start_local_compiler(&solution_name, &project_name, &compiler_path, &replica_working_dir, &compiler_commands, &stdout_err_stream);

    drop(stdout_err_stream);

    let compile_output = String::from_utf8_lossy(&stdout);
    let compile_error = String::from_utf8_lossy(&stderr);
    
    log::info!("injectd compile status: {}, {:?} stdout: {:?} stderr: {:?}", status, now.elapsed(), compile_output, compile_error);

    if status == 0 {
        let lines: Vec<_> = compile_output.lines().collect();
        let (files, _warnings) = filter_compiler_warning(lines);
        if unready_objfiles.lock().unwrap().is_empty() {
            let out_stream = out_err_stream.stdout.clone();
            for line in files {
                let _ = pre_return_local_compile_result_objfiles(&std::borrow::Cow::from(line), generated_object.clone(), &solution_name, &origin_working_dir, &out_stream);
            }
            drop(out_stream);
        }
        else {
            return_local_compile_result_objfiles(unready_objfiles, &solution_name, &origin_working_dir, out_err_stream);
        }
        return_local_compile_result_pdbfiles(program_database, &solution_name, &origin_working_dir, out_err_stream);
    }
    else {
        let lines: Vec<&str> = compile_output.lines().collect();
        let (lines, errors) = filter_compiler_error(lines);
        let compiled_filename: Vec<_> = lines.iter().map(|item| std::ffi::OsString::from(item)).collect();
        let compiled_output: Vec<_> = errors.iter().map(|item| std::ffi::OsString::from(item)).collect();
        log::warn!("compile result failed. filename: {:?}, errors: {:?}", compiled_filename, compiled_output);
    }
    
    let result = CompilerOutput {
        status: status,
        out: stdout,
        err: stderr,
    };

    return (result, None);
}

fn pre_return_local_compile_result_objfiles(line: &std::borrow::Cow<'_, str>, generated_object: GeneratedObject, solution_name: &std::ffi::OsString, 
                                                origin_working_dir: &std::ffi::OsString, out_stream: &std::sync::mpsc::Sender<CompiledResults>) 
                                                -> std::option::Option<(std::string::String, std::path::PathBuf)> {

    let mut unready_objfiles: std::option::Option<(std::string::String, std::path::PathBuf)> = None;

    let line = line.replace(r#"""#, "").trim_end().to_string();
    if line.starts_with("Generating Code...") { 
    
    }
    else if  line.ends_with(".i") || line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".cc") {

        let mut compiled_results: CompiledResults = Vec::new();
        let mut obj: Option<(std::ffi::OsString, Vec<u8>)> = None;

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

                let origin = repair_original_path(&solution_name, &origin_working_dir, &result);        

                obj = Some((origin, contents));
            },
            Err(error) => {
                unready_objfiles = Some((line.clone(), result.clone()));
                if error.kind() == std::io::ErrorKind::NotFound {
                    log::trace!(".obj file path is not found. {:?}.", result);
                }
                else {
                    log::error!(".obj file read failed. {:?}, {:?}.", error, result);
                }
            }
        };

        result.clear();

        if obj.is_some() {
            let compiled_result = CompiledResult {
                source_file: std::ffi::OsString::from(&line),
                obj: obj,
                pdb: None,
                idb: None,
            };
            compiled_results.push(compiled_result);
        
            let _ = out_stream.send(compiled_results).unwrap_or_else(|err| {
                log::warn!("send .obj results to out stream failed: {:?}", err);
            });
        }
    }
    else {
        log::trace!("exclude source file, maybe warning and error. {:?}", line);
    }
    return unready_objfiles;
}

fn return_local_compile_result_objfiles(objfiles: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<std::string::String, std::path::PathBuf>>>, solution_name: &std::ffi::OsString, origin_working_dir: &std::ffi::OsString, out_err_stream: &crate::compiler::msvc::CompiledResultsStream) {
    
    log::trace!("unready obj files: {:?}", objfiles.lock().unwrap());
    let objfiles: Vec<_> = objfiles.lock().unwrap().iter().map(|(k, v)|(k.clone(), v.clone())).collect();
    for (line, objfile) in objfiles {

        let out_stream = out_err_stream.stdout.clone();
        let solution_name_ = solution_name.clone();
        let origin_working_dir_ = origin_working_dir.clone();

        let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {
            let mut compiled_results: CompiledResults = Vec::new();
            let mut obj: Option<(std::ffi::OsString, Vec<u8>)> = None;
            
            match std::fs::File::open(&objfile) {
                Ok(file) => {
                    log::info!("unready obj file: {:?}", objfile);

                    let mut contents = Vec::new();
                    let mut file = std::io::BufReader::new(file);
                    let _ = file.read_to_end(&mut contents).unwrap();

                    let origin = repair_original_path(&solution_name_, &origin_working_dir_, &objfile);        

                    obj = Some((origin, contents));
                },
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        log::trace!("unready .obj file path is not found. {:?}.", objfile);
                    }
                    else {
                        log::error!("unready .obj file read failed. {:?}, {:?}.", error, objfile);
                    }
                }
            };

            let compiled_result = CompiledResult {
                source_file: std::ffi::OsString::from(&line),
                obj: obj,
                pdb: None,
                idb: None,
            };
            compiled_results.push(compiled_result);
            
            if !compiled_results.is_empty() {
                let _ = out_stream.send(compiled_results).unwrap_or_else(|err| {
                    log::warn!("send unready .obj results to out stream failed: {:?}", err);
                });
            }
            drop(out_stream);
        });
    }
    log::trace!("unready obj file end, send out stream end.");
}

fn return_local_compile_result_pdbfiles(program_database: ProgramDataBase, solution_name: &std::ffi::OsString, origin_working_dir: &std::ffi::OsString, out_err_stream: &crate::compiler::msvc::CompiledResultsStream) {
    let mut result = std::path::PathBuf::from("");
    match &program_database {
        ProgramDataBase::PathWithPDBName(path) => {
            result = path.clone();
        },
        ProgramDataBase::PathWithoutPDBName(dir) => {
            result = dir.join("vc143");
            result.set_extension("pdb");
        },
        _ => {
            log::warn!("fetch result pdb file path failed.");
        }
    };
    
    log::trace!("compile result program database path: {:?}", result);
    let mut compiled_results: CompiledResults = Vec::new();

    let mut pdb: Option<(std::ffi::OsString, Vec<u8>)> = None;
    let mut idb: Option<(std::ffi::OsString, Vec<u8>)> = None;

    if result.exists() {
        match std::fs::File::open(&result) {
            Ok(file) => {
                let mut contents = Vec::new();
                let mut file = std::io::BufReader::new(file);
                let _ = file.read_to_end(&mut contents).unwrap();
                let origin = repair_original_path(&solution_name, &origin_working_dir, &result);        
                pdb = Some((origin, contents));
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

        result.set_extension("idb");
        match std::fs::File::open(&result) {
            Ok(file) => {
                let mut contents = Vec::new();
                let mut file = std::io::BufReader::new(file);
                let _ = file.read_to_end(&mut contents).unwrap();
                let origin = repair_original_path(&solution_name, &origin_working_dir, &result);     
                idb = Some((origin, contents));
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
        if pdb.is_some() || idb.is_some() {
            
            // .ilk .res .asm
            let compiled_result = CompiledResult {
                source_file: std::ffi::OsString::from(&result),
                obj: None,
                pdb: pdb,
                idb: idb,
            };
            compiled_results.push(compiled_result);
        
            out_err_stream.stdout.send(compiled_results).unwrap_or_else(|err| {
                log::warn!("send compiled pdb results to out stream failed: {:?}", err);
            });
        }
    }
    else {
        log::warn!("pdb file is not found, path: {:?}.", result);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum GeneratedObject {
    NoneObjPath,
    PathWithObjName(std::path::PathBuf),
    PathWithoutObjName(std::path::PathBuf),
}

fn filter_compiler_warning(lines: Vec<&str>) -> (Vec<&str>, Vec<&str>) {
    let (files, warning):(Vec<_>, Vec<_>) = lines.into_iter().partition(|item| item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c") || item.ends_with(".cc"));
    return (files, warning);
}

fn filter_compiler_error(lines: Vec<&str>) -> (Vec<&str>, Vec<&str>) {
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
}

fn exact_compiler_object_file(project: std::borrow::Cow<str>, mut arg: std::borrow::Cow<str>, working_dir: &std::path::PathBuf) -> GeneratedObject {

    let replica = tools::utils::access_replica_dir();
    let replica = std::path::PathBuf::from(replica);

    let split = |obj: std::path::PathBuf, project: std::borrow::Cow<str>| {
        if project.is_empty() {
            return obj;
        }
        else {
        
            let components = obj.components().collect::<Vec<_>>();
            if let Some(index) = components.iter().position(|item| item.as_os_str().to_string_lossy() == project) {
                let result: std::path::PathBuf = components[index + 1..].iter().collect();
                let path = replica.join("Project").join(project.into_owned()).join(result);
                return path;
            }
            else {
                return obj;
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
            let path = working_dir.join(path);
            let path = split(path, project.clone());
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

fn exact_compile_pdb_path(solution: std::borrow::Cow<str>, mut arg: std::borrow::Cow<str>, working_dir: &std::path::PathBuf) -> ProgramDataBase {

    let replica = tools::utils::access_replica_dir();
    let replica = std::path::PathBuf::from(replica);

    let split = |pdb: std::path::PathBuf, solution: std::borrow::Cow<str>| {
        if solution.is_empty() {
            return pdb;
        }
        else {
            let components = pdb.components().collect::<Vec<_>>();
            if let Some(index) = components.iter().position(|item| item.as_os_str().to_string_lossy() == solution) {
                let result: std::path::PathBuf = components[index + 1..].iter().collect();
                let path = replica.join("Project").join(solution.into_owned()).join(result);
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
            let path = split(path, solution.clone());
            let path = replica.join("Project").join(solution.into_owned()).join(path);
            return ProgramDataBase::PathWithPDBName(path);
        }
        else {
            let middle = split(working_dir.to_owned(), solution.clone());
            let path = replica.join("Project").join(solution.into_owned()).join(middle).join(path);
            return ProgramDataBase::PathWithPDBName(path);
        }
    }
    else {
        let path = std::path::PathBuf::from(pdb);
        if path.has_root() {
            let path = split(path, solution.clone());
            let path = replica.join("Project").join(solution.into_owned()).join(path);
            return ProgramDataBase::PathWithoutPDBName(path);
        }
        else {
            let middle = split(working_dir.to_owned(), solution.clone());
            let path = replica.join("Project").join(solution.into_owned()).join(middle).join(path);
            return ProgramDataBase::PathWithoutPDBName(path);
        }
    }
}

struct CompileAction {
    pub program_database: ProgramDataBase,
    pub generated_object: GeneratedObject,
    pub source_files: std::collections::HashMap<std::string::String, std::path::PathBuf>,
}

fn parse_action_from_commands(compiler_input: &CompilerInput) -> CompileAction {

    let solution_name = compiler_input.solution.clone();

    let mut pdb = ProgramDataBase::NonePDBPath;
    let mut obj = GeneratedObject::NoneObjPath;
    let mut sourcefiles: std::collections::HashMap<String, std::path::PathBuf> = std::collections::HashMap::new();
    let working_dir = std::path::PathBuf::from(compiler_input.compiler_working_dir.clone());

    if compiler_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("CMake") 
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("Dist") {
        
        let mut default_pdb = false;
        for command in &compiler_input.compiler_commands {
            let command = command.to_string_lossy();
            if command.starts_with("/Fd") {
                pdb = exact_compile_pdb_path(solution_name.to_string_lossy(), command, &working_dir);
            }
            else if command.starts_with("/Zi") {
                default_pdb = true;
            }
            else if command.starts_with("/Fo") {
                obj = exact_compiler_object_file(solution_name.to_string_lossy(), command, &working_dir);
            }
            else if command.to_lowercase().ends_with(".i") || command.to_lowercase().ends_with(".cpp") || command.to_lowercase().ends_with(".c") || command.to_lowercase().ends_with(".cc") {
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
                            sourcefiles.insert(source_file_name, source_path);
                        }
                        else {
                            let source_path = working_dir.join(source.clone());
                            sourcefiles.insert(source_file_name, source_path);
                        }
                    },
                    None => {
                        let absolute_source_path = working_dir.join(source.clone());
                        sourcefiles.insert(source.clone(), absolute_source_path);
                    }
                }
            }
        }

        if pdb == ProgramDataBase::NonePDBPath && default_pdb {
            let replica = tools::utils::access_replica_dir();
            let replica = std::path::PathBuf::from(replica);
            pdb = ProgramDataBase::PathWithPDBName(replica.join("Project").join(solution_name).join("vc143.pdb"));
        }
    }
    else {
        
    }
    return CompileAction { program_database: pdb, generated_object: obj, source_files: sourcefiles };
}

fn start_local_compiler_with_inject(solution: &std::ffi::OsString, project: &std::ffi::OsString, compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, 
                            compiler_commands: &Vec<std::ffi::OsString>, out_err_stream: &OutAndErrStream) -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {
    
    let line: String = compiler_commands.clone().into_iter()
        .map(|os_string| format!("{} ", os_string.to_string_lossy()))
        .collect();

    let line =  format!(r#""{}" {}"#, compiler_path.to_string_lossy(), line);

    let (status, stdout, stderr) = crate::detours::redirect::msvc_detours(solution.clone().into_string().unwrap(), project.clone().into_string().unwrap(), compiler_path.clone().into_string().unwrap(), 
                    line, working_dir.clone().into_string().unwrap(), out_err_stream);

    return (status, stdout, stderr);
}

fn start_local_compiler(solution: &std::ffi::OsString, project: &std::ffi::OsString, compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, compiler_commands: &Vec<std::ffi::OsString>, out_err_stream: &OutAndErrStream) 
                    -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {

    log::trace!("solution: {:?}", solution);
    log::trace!("project: {:?}", project);
    log::trace!("working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compile content: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    
    let (status, stdout, stderr) = start_local_compiler_with_inject(solution, project, compiler_path, working_dir, compiler_commands, out_err_stream);
    
    let elapsed = start.elapsed();
    log::info!("compile file with inject elapsed time: {:?}.", elapsed);
    return (status, stdout, stderr);
}

//TODO: tokio::net::windows::named_pipe
pub fn redirect_stdout_log() {
    use tokio::io::AsyncWriteExt;
    log::info!("redirect stdout log loop thread start.");

    use std::os::windows::ffi::OsStrExt;
    let os_string = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");

    let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
    wchars.push(0);

    let mut count  = 0;
    unsafe { loop {

        let pipe = winapi::um::namedpipeapi::CreateNamedPipeW(wchars.as_ptr(), winapi::um::winbase::PIPE_ACCESS_INBOUND,  
        winapi::um::winbase::PIPE_TYPE_MESSAGE | winapi::um::winbase::PIPE_READMODE_MESSAGE |  winapi::um::winbase::PIPE_WAIT,
        winapi::um::winbase::PIPE_UNLIMITED_INSTANCES,
        0, 0, 0, std::ptr::null_mut());
        
        if !pipe.is_null() && pipe != winapi::um::handleapi::INVALID_HANDLE_VALUE {
            
            if winapi::shared::minwindef::TRUE == winapi::um::namedpipeapi::ConnectNamedPipe(pipe, std::ptr::null_mut()) {

                let handle = tools::ptr::HandleBox::new(pipe);
                let _ = crate::common::COCREW_RUNTIME.lock().unwrap().spawn(async move {
                //let _ = std::thread::spawn(move || {

                    log::info!("redirect stdout log read named pipe message task start. count: {}", count);
                    let mut buffer = vec![0u8; 512];
                    let mut bytes: winapi::shared::minwindef::DWORD = 0;
                    let mut moredata = String::new();
                    loop {
                        let result = winapi::um::fileapi::ReadFile(
                            handle.get().to_owned(),
                            buffer.as_mut_ptr() as *mut _,
                            512,
                            &mut bytes,
                            std::ptr::null_mut()
                        );
        
                        if result == winapi::shared::minwindef::FALSE || bytes == 0 {
                            let error = winapi::um::errhandlingapi::GetLastError();

                            if error == winapi::shared::winerror::ERROR_BROKEN_PIPE {
                                //The pipe has been ended.
                                break;
                            }
                            else if error == winapi::shared::winerror::ERROR_MORE_DATA {
                                //The buffer is not enough.
                                let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                                moredata.push_str(&output);
                                continue;
                            }
                            else if error == winapi::shared::winerror::ERROR_IO_PENDING {
                                //The operation is pending.
                                continue;
                            }
                            else {
                                log::warn!("reaf pipe failed, error code: {}, message: {}, count: {}", error, tools::utils::get_winapi_error_message(error), count);
                                break;
                            }
                        }
                        let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                        if moredata.is_empty() {
                            log::info!("redirect: {}", output);
                        }
                        else {
                            moredata.push_str(&output);
                            log::info!("redirect: {}", moredata);
                            moredata.clear();
                        }
                        //tokio::io::stdout().write_all(format!("redirect: {}\n", output).as_bytes()).await.expect("Failed to write to stdout");
                    }
                    winapi::um::namedpipeapi::DisconnectNamedPipe(handle.get().to_owned());
                    winapi::um::handleapi::CloseHandle(handle.get().to_owned());
                    log::warn!("redirect stdout log read named pipe message task exit. count: {}", count);
                });
            }
            else {
                let error = winapi::um::errhandlingapi::GetLastError();
                log::debug!("connect named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);

                if error == winapi::shared::winerror::ERROR_NO_DATA {
    
                }
                else {
        
                }
                winapi::um::handleapi::CloseHandle(pipe);
            }
        }
        else {
            let error = winapi::um::errhandlingapi::GetLastError();
            log::error!("connect named pipe failcreate named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);
        }
        count += 1;
    }}
}

fn repair_original_path(solution: &std::ffi::OsString, working_dir: &std::ffi::OsString, path: &std::path::PathBuf) -> std::ffi::OsString {

    //get the original .obj/.pdb file path
    let components = path.components().collect::<Vec<_>>();
    if let Some(index) = components.iter().position(|item| item.as_os_str().to_string_lossy() == solution.to_string_lossy()) {
        let result: std::path::PathBuf = components[index + 1..].iter().collect();
        
        let base = std::path::PathBuf::from(working_dir);

        let base = base.components().collect::<Vec<_>>();
        if let Some(index) = base.iter().position(|item| item.as_os_str().to_string_lossy() == solution.to_string_lossy()) {
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

fn redirect_compiler_path(compiler: std::ffi::OsString) -> Option<std::ffi::OsString> {

    match compiler.to_string_lossy().find("MSVC") {
        Some(index) => {
            let mut path = compiler.to_string_lossy().to_string();
            let path = std::path::PathBuf::from(format!(r#"{}\{}"#, tools::utils::access_replica_dir(), path.split_off(index)));
            if std::fs::exists(&path).unwrap() {
                return Some(path.into_os_string());
            }
            else {
                return None;
            }
        },
        None => return None,
    };
} 

fn redirect_working_dir(working_dir: &std::ffi::OsString, solution: &std::ffi::OsString) -> Option<std::ffi::OsString> {
    if let Some(index) = working_dir.to_string_lossy().find(solution.to_str().unwrap()) {
        let dir = tools::utils::access_working_path("Replica").unwrap_or_default();
        let path = std::path::PathBuf::from(format!(r#"{}\Project\{}"#, dir, working_dir.to_string_lossy().to_string().split_off(index)));
        if std::fs::exists(&path).unwrap() {
            return Some(path.into_os_string());
        }
        else {
            log::warn!("redirect working dir not exist so create, path: {:?} is not exists.", path);
            std::fs::create_dir_all(&path).unwrap_or_else(|err| {
                log::error!("create redirect working dir failed: {:?}, error: {:?}", path, err);
            });
            return Some(path.into_os_string());
        }
    }
    else {
        return None;
    }
} 

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn compile_sourcefile_with_inject_test() {
        println!("run msvc .c file compile test with inject");
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

        let (out_sender, out_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let (err_sender, err_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    
        let task = std::thread::spawn(move || {
            while let Ok(data) = out_receiver.recv() {
                println!("stream stdout: {:?}", String::from_utf8_lossy(&data));
            }
        
            while let Ok(data) = err_receiver.recv() {
                println!("stream stderr: {:?}", String::from_utf8_lossy(&data));
            }
        });

        let out_err_stream = crate::compiler::msvc::OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), &std::ffi::OsString::new(),
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands, &out_err_stream);
        
        drop(out_err_stream);

        task.join().unwrap();
        assert!(status == 0);
        println!("compile stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile stderr: {}", String::from_utf8_lossy(&stderr));
    }

    #[test]
    fn compile_preprocessed_file_with_inject_test() {
        println!("run msvc .i file compile test with inject");
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

        let (out_sender, out_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let (err_sender, err_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    
        let out_err_stream = crate::compiler::msvc::OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), &std::ffi::OsString::new(), 
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands, &out_err_stream);
        assert!(status == 0);
        println!("compile .i file stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile .i file stderr: {}", String::from_utf8_lossy(&stderr));
        assert!(status == 0);
    }
    
    #[test]
    fn compile_preprocessed_file_use_project_arg() {
        println!("run msvc compile use project args test with inject, if you want simulate the project compile, please use this test and replace args.");

        tools::logger::init_once_logger();

        let _handle = std::thread::spawn(||{
            redirect_stdout_log();
        });


        let compiler_commands: Vec<std::ffi::OsString> = vec! ["/c", "/I \"E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\"", "/I \"E:\\TestFuture\\GammaRay\\GammaRayTool\\3rdparty\\kde\"", "/I \"E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\\gammaray_kitemmodels_autogen\\include_Debug\"", "/I \"E:\\TestFuture\\GammaRay\\GammaRayTool\"", "/I \"E:\\TestFuture\\GammaRay\\GammaRayTool\\3rdparty\"", "/I \"E:\\TestFuture\\GammaRay\\GammaRayTool\\build\"", "/Zi", "/nologo", "/W1", "/WX-", "/diagnostics:column", "/Od", "/Ob0", "/D", "_WINDLL", "/D", "_MBCS", "/D", "WIN32", "/D", "_WINDOWS", "/D", "QT_DISABLE_DEPRECATED_BEFORE=0x050500", "/D", "QT_USE_FAST_CONCATENATION", "/D", "QT_USE_FAST_OPERATOR_PLUS", "/D", "QT_NO_CAST_TO_ASCII", "/D", "QT_NO_URL_CAST_FROM_STRING", "/D", "QT_CORE_LIB", "/D", "CMAKE_INTDIR=\\\"Debug\\\"", "/D", "MAKE_KITEMMODELS_LIB", "/Gm-", "/EHsc", "/RTC1", "/MDd", "/GS", "/fp:precise", "/Zc:wchar_t", "/Zc:forScope", "/Zc:inline", "/GR", "/Fogammaray_kitemmodels.dir\\Debug\\", "/Fdgammaray_kitemmodels.dir\\Debug\\vc143.pdb", "/external:W0", "/Gd", "/TP", "/wd4244", "/wd4267", "/errorReport:prompt", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtCore", "/external:I", "D:/WorkTool/Qt/qt_5.15.2.17/out64/./mkspecs/win32-msvc", "/TP", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\\gammaray_kitemmodels.dir\\Debug\\mocs_compilation_Debug.i", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\\gammaray_kitemmodels.dir\\Debug\\klinkitemselectionmodel.i", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\\gammaray_kitemmodels.dir\\Debug\\kmodelindexproxymapper.i", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\\gammaray_kitemmodels.dir\\Debug\\kdescendantsproxymodel.i", "E:\\TestFuture\\GammaRay\\GammaRayTool\\build\\3rdparty\\kde\\gammaray_kitemmodels.dir\\Debug\\kitemmodels_debug.i"]
            .iter()
            .map(|item|std::ffi::OsString::from(*item)).collect::<Vec<std::ffi::OsString>>();

        let env = crew::platform::windows::WindowsCompilerEnv::default();
        let mut complier_path = env.compiler_path;
        complier_path.push(r"Hostx64\x64\cl.exe");

        let mut working_dir = std::ffi::OsString::from("C:\\WorkSpace\\TurboBuildTool\\turbobuild\\Replica\\Project\\GammaRayTool\\build\\3rdparty\\kde");
        
        let (out_sender, out_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let (err_sender, err_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    
        let stdout_err_stream = OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::from("GammaRayTool"), &std::ffi::OsString::new(),
            &complier_path.into_os_string(), &working_dir, &compiler_commands, &stdout_err_stream);
        log::info!("stdout: {}", String::from_utf8_lossy(&stdout));
        log::info!("stderr: {}", String::from_utf8_lossy(&stderr));
        assert!(status == 0);
    }

    #[test]
    fn compile_preprocessed_file_use_project_arg_mp() {
        println!("run msvc compile use project args test with inject, if you want simulate the project compile, please use this test and replace args.");

        tools::logger::init_once_logger();

        let _handle = std::thread::spawn(||{
            redirect_stdout_log();
        });

        // must contain "/MP" arg, /FD must use absolute path, not relative path.
        let compiler_commands: Vec<std::ffi::OsString> = vec!["/c", "/I \"G:\\OpenSource\\llvm-project\\build\\lib\\Support\\BLAKE3\"", "/I \"G:\\OpenSource\\llvm-project\\llvm\\lib\\Support\\BLAKE3\"", "/I \"G:\\OpenSource\\llvm-project\\build\\include\"", "/I \"G:\\OpenSource\\llvm-project\\llvm\\include\"", 
        "/Zi", "/nologo", "/W4", "/WX-", "/diagnostics:column", "/MP", "/Od", 
        "/Ob0", "/Oi", "/D", "_UNICODE", "/D", "UNICODE", "/D", "WIN32", "/D", "_WINDOWS", "/D", "_HAS_EXCEPTIONS=0", "/D", "GTEST_HAS_RTTI=0", "/D", "_CRT_SECURE_NO_DEPRECATE", 
        "/D", "_CRT_SECURE_NO_WARNINGS", "/D", "_CRT_NONSTDC_NO_DEPRECATE", "/D", "_CRT_NONSTDC_NO_WARNINGS", "/D", "_SCL_SECURE_NO_DEPRECATE", "/D", "_SCL_SECURE_NO_WARNINGS", "/D", "UNICODE", 
        "/D", "_UNICODE", "/D", "__STDC_CONSTANT_MACROS", "/D", "__STDC_FORMAT_MACROS", "/D", "__STDC_LIMIT_MACROS", "/D", "CMAKE_INTDIR=\\\"Debug\\\"", "/Zc:preprocessor", 
        "/Gm-", "/RTC1", "/MDd", "/GS", "/fp:precise", "/Zc:wchar_t", "/Zc:forScope", "/Zc:inline", "/permissive-", "/FoLLVMSupportBlake3.dir\\Debug\\", 
        "/FdF:\\OpenSource\\llvm-project\\build\\Debug\\lib\\LLVMSupportBlake3.pdb", "/external:W4", "/Gd", "/TC", "/wd4141", "/wd4146", "/wd4244", "/wd4267", "/wd4291", "/wd4351", "/wd4456", "/wd4457", 
        "/wd4458", "/wd4459", "/wd4503", "/wd4624", "/wd4722", "/wd4100", "/wd4127", "/wd4512", "/wd4505", "/wd4610", "/wd4510", "/wd4702", "/wd4245", "/wd4706", "/wd4310", "/wd4701", 
        "/wd4703", "/wd4389", "/wd4611", "/wd4805", "/wd4204", "/wd4577", "/wd4091", "/wd4592", "/wd4319", "/wd4709", "/wd5105", "/wd4324", "/wd4251", "/wd4275", "/errorReport:prompt", 
        "/we4238", "-w14062", "/Gw", 
        "F:\\OpenSource\\llvm-project\\build\\lib\\Support\\BLAKE3\\LLVMSupportBlake3.dir\\Debug\\blake3.i",
        "F:\\OpenSource\\llvm-project\\build\\lib\\Support\\BLAKE3\\LLVMSupportBlake3.dir\\Debug\\blake3_dispatch.i",
        "F:\\OpenSource\\llvm-project\\build\\lib\\Support\\BLAKE3\\LLVMSupportBlake3.dir\\Debug\\blake3_portable.i",
        "F:\\OpenSource\\llvm-project\\build\\lib\\Support\\BLAKE3\\LLVMSupportBlake3.dir\\Debug\\blake3_neon.i"
         ]
            .iter()
            .map(|item|std::ffi::OsString::from(*item)).collect::<Vec<std::ffi::OsString>>();


        let complier_path = std::path::PathBuf::from(r"D:\turbobuild\target\debug\Replica\MSVC\14.39.33519\bin\Hostx64\x64\cl.exe");

        let working_dir = std::ffi::OsString::from(r"D:\turbobuild\target\debug\Replica\\Project\\llvm-project\\build\\lib\\Support\\BLAKE3");
        
        let (out_sender, out_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let (err_sender, err_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    
        let stdout_err_stream = OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::from("llvm-project"), &std::ffi::OsString::from(""), 
            &complier_path.into_os_string(), &working_dir, &compiler_commands, &stdout_err_stream);
        log::info!("stdout: {}", String::from_utf8_lossy(&stdout));
        log::info!("stderr: {}", String::from_utf8_lossy(&stderr));
        assert!(status == 0);
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