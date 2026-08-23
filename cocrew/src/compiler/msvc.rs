
use crew::compiler::model::{CompiledResult, CompiledResults, CompilerInput, CompilerOutput};
use std::io::Read;
use windows_sys::Win32 as win;

pub struct MSVC {
    pub version: String,
    pub out_err_stream: crate::compiler::msvc::CompiledResultsStream,
}

pub struct OutAndErrStream {
    pub stdout: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub stderr: tokio::sync::mpsc::Sender<Vec<u8>>,
}

pub struct CompiledResultsStream {
    pub stdout: tokio::sync::mpsc::Sender<crew::compiler::model::CompiledResults>,
    pub stderr: tokio::sync::mpsc::Sender<crew::compiler::model::CompiledResults>,
}

static COMPILER_VERSION_MAP_PATH_CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<std::string::String, std::ffi::OsString>>> = std::sync::LazyLock::new(|| {
    let map = std::collections::HashMap::new();
    std::sync::Mutex::new(map)
});

pub struct CompileTaskCount {
    pub pdb: ProgramDataBase,
    pub expected: usize,
    pub done: usize,
}

pub static COMPILE_TASK_COUNT: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, CompileTaskCount>>>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())));

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
        input.envs = compiler_input.envs.clone();

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
    let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);

    let stdout_err_stream = OutAndErrStream {
        stdout: out_sender,
        stderr: err_sender,
    };
    
    let solution_name = std::sync::Arc::new(compiler_input.solution.clone());
    let solution_name_ = std::sync::Arc::clone(&solution_name);

    let project_name = std::sync::Arc::new(compiler_input.project.clone());

    let compiler_path = compiler_input.compiler_path.clone();
    let replica_working_dir = compiler_input.compiler_working_dir.clone();
    let compiler_commands = compiler_input.compiler_commands.clone();
    let envs = compiler_input.envs.clone();

    let actions = parse_action_from_commands(&compiler_input);
    let generated_object = std::sync::Arc::new(actions.generated_object);
    let program_database = std::sync::Arc::new(actions.program_database);

    let out_stream = out_err_stream.stdout.clone();

    let project_name_ = project_name.clone();
    let project_name__ = project_name_.clone();
    let origin_working_dir_ = origin_working_dir.clone();
    let generated_object_ = generated_object.clone();

    let stream_objfiles = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let unready_objfiles = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let unready_objfiles_ = unready_objfiles.clone();
    let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};
    let rt_  = rt.clone();
    let _ = rt.spawn(async move {

        while let Some(data) = out_receiver.recv().await {
            stream_objfiles.lock().unwrap().push(data.clone());
            let line = String::from_utf8_lossy(&data);

            log::info!("{:?} stream stdout: {:?}", &project_name_, line);

            /* 
            //limited concurrency
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(32));

            line.lines().for_each(|item| {
                let item = item.to_string();
                let generated_object_ = generated_object_.clone();
                let solution_name_ = std::sync::Arc::clone(&solution_name_);
                let origin_working_dir_ = origin_working_dir_.clone();
                let out_stream = out_stream.clone();
                let unready_objfiles_ = unready_objfiles_.clone();
                let semaphore_ = semaphore.clone();

                rt_.spawn(async move {
                    if !item.is_empty() {
                        let permit = semaphore_.clone().acquire_owned().await.unwrap();
                        let objfile = pre_return_local_compile_result_object_files(&std::borrow::Cow::from(item), &(*generated_object_), &solution_name_, &origin_working_dir_, &out_stream).await;
                        if let Some(objfile) = objfile {
                            unready_objfiles_.lock().unwrap().insert(objfile.0, objfile.1);
                        }
                        drop(permit);
                    }
                });
            });
            */
        }

        drop(out_stream);
        log::info!("stream stdout end {:?}", &project_name_);
    });

    let err_stream = out_err_stream.stderr.clone();

    let _ = rt.spawn(async move {
        while let Some(data) = err_receiver.recv().await {
            log::info!("{:?} stream stderr: {:?}",  &project_name__, String::from_utf8_lossy(&data));

            //let _ = err_stream.send(data);
        }
        log::info!("stream stderr end {:?}", &project_name__);
        drop(err_stream);
    });

    let project_name_ = project_name.clone();
    let (status, stdout, stderr) = start_local_compiler(&solution_name, &project_name, &compiler_path, &replica_working_dir, &compiler_commands, &envs, &stdout_err_stream);

    drop(stdout_err_stream);

    let compile_output = String::from_utf8_lossy(&stdout);
    let compile_error = String::from_utf8_lossy(&stderr);
    
    log::info!("{:?} injectd compile status: {}, {:?} stdout: {:?} stderr: {:?}", project_name_, status, now.elapsed(), compile_output, compile_error);

    if status == 0 {
        let lines: Vec<_> = compile_output.lines().collect();
        let (files, _warnings) = filter_compiler_warning(lines);
        if unready_objfiles.lock().unwrap().is_empty() {
            /* 
            let out_stream = out_err_stream.stdout.clone();
            for line in files {
                let out_stream_ = out_stream.clone();
                let generated_object = generated_object.clone();
                let line = line.to_string();
                let solution_name = solution_name.clone();
                let origin_working_dir = origin_working_dir.clone();
                rt.spawn(async move {
                    let _ = pre_return_local_compile_result_object_files(&std::borrow::Cow::from(line), &(*generated_object), &solution_name, &origin_working_dir, &out_stream_);
                });
            }
            drop(out_stream);
            */
        }
        else {
            return_local_compile_result_object_files(unready_objfiles, &solution_name, &origin_working_dir, out_err_stream);
        }

        let mut synced_tasks = COMPILE_TASK_COUNT.lock().unwrap();
        let task = synced_tasks.get_mut(&project_name.to_string_lossy().to_string());
        if let Some(task) = task {
            task.pdb = (*program_database).clone();
            task.done += 1;
            log::trace!("compile task expected: {:?} done: {:?}, project: {:?}.", task.expected, task.done, &project_name);
            if task.expected != 0 && task.done >= task.expected {
                synced_tasks.remove(&project_name.to_string_lossy().to_string());
                drop(synced_tasks);
                return_local_compile_result_pdb_files((*program_database).clone(), &solution_name, &origin_working_dir, out_err_stream);
            }
        }
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

async fn pre_return_local_compile_result_object_files(line: &std::borrow::Cow<'_, str>, generated_object: &GeneratedObject, solution_name: &std::ffi::OsString, 
                                                origin_working_dir: &std::ffi::OsString, out_stream: &tokio::sync::mpsc::Sender<CompiledResults>) 
                                                -> std::option::Option<(std::string::String, std::path::PathBuf)> {

    use tokio::io::AsyncReadExt;
    let mut unready_objfiles: std::option::Option<(std::string::String, std::path::PathBuf)> = None;

    let line = line.replace(r#"""#, "");
    let line = line.trim_end();
    
    if line.starts_with("Generating Code...") { 
    
    }
    else if line.ends_with(".i") || line.ends_with(".cpp") || line.ends_with(".c") || line.ends_with(".cc") || line.ends_with(".cxx") {

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

        let compiled_result = crew::compiler::model::CompiledResult {
            source_file: std::ffi::OsString::from(&line),
            obj: None,
            pdb: None,
            idb: None,
        };
        let _ = out_stream.try_send(vec![compiled_result]).unwrap_or_else(|err| {
            log::warn!("try send .obj results to out stream failed: {:?}", err);
        });
        
        match tokio::fs::File::open(&result).await {
            Ok(file) => {
                let mut contents = Vec::new();
                let mut file = tokio::io::BufReader::new(file);
                let _ = file.read_to_end(&mut contents).await.unwrap();

                let origin = repair_original_path(&solution_name, &origin_working_dir, &result);        

                let compiled_gen_result = crew::compiler::model::CompiledResult {
                    source_file: std::ffi::OsString::from(&line),
                    obj: Some((origin, -1, bytes::Bytes::from(contents))),
                    pdb: None,
                    idb: None,
                };

                crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                    log::warn!("send unready .obj file path to channel failed: {:?}", err);
                });
            },
            Err(error) => {
                unready_objfiles = Some((line.to_string(), result.clone()));
                if error.kind() == std::io::ErrorKind::NotFound {
                    log::trace!(".obj file path is not found. {:?}.", unready_objfiles);
                }
                else {
                    log::error!(".obj file read failed. {:?}, {:?}.", error, unready_objfiles);
                }
            }
        };
        result.clear();
    }
    else {
        log::trace!("exclude source file, maybe warning and error. {:?}", line);
    }
    return unready_objfiles;
}

fn return_local_compile_result_object_files(objfiles: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<std::string::String, std::path::PathBuf>>>, solution_name: &std::ffi::OsString, origin_working_dir: &std::ffi::OsString, out_err_stream: &crate::compiler::msvc::CompiledResultsStream) {
    
    log::trace!("unready obj files: {:?}", objfiles.lock().unwrap());
    use tokio::io::AsyncReadExt;
    let objfiles: Vec<_> = objfiles.lock().unwrap().iter().map(|(k, v)|(k.clone(), v.clone())).collect();
    for (line, objfile) in objfiles {

        let out_stream = out_err_stream.stdout.clone();
        let solution_name_ = solution_name.clone();
        let origin_working_dir_ = origin_working_dir.clone();

        let compiled_result = CompiledResult {
            source_file: std::ffi::OsString::from(&line),
            obj: None,
            pdb: None,
            idb: None,
        };
        let _ = out_stream.try_send(vec![compiled_result]).unwrap_or_else(|err| {
            log::warn!("send unready .obj results to out stream failed: {:?}", err);
        });

        let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};
        let _ = rt.spawn(async move {

            match tokio::fs::File::open(&objfile).await {
                Ok(file) => {
                    log::info!("sync unready obj file: {:?}", objfile);

                    let mut contents = Vec::new();
                    let mut file = tokio::io::BufReader::new(file);
                    let _ = file.read_to_end(&mut contents).await.unwrap();

                    let origin = repair_original_path(&solution_name_, &origin_working_dir_, &objfile);


                    let compiled_gen_result = crew::compiler::model::CompiledResult {
                        source_file: std::ffi::OsString::from(&line),
                        obj: Some((origin, -1,  bytes::Bytes::from(contents))),
                        pdb: None,
                        idb: None,
                    };
                    crate::communicate::unpackager::TASK_TO_FILE_CHANNEL.task_to_file_tx.send(compiled_gen_result).await.unwrap_or_else(|err| {
                        log::warn!("send unready .obj file path to channel failed: {:?}", err);
                    });
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
        });
    }
    log::trace!("unready obj file end, send out stream end.");
}

fn return_local_compile_result_pdb_files(program_database: ProgramDataBase, solution_name: &std::ffi::OsString, origin_working_dir: &std::ffi::OsString, out_err_stream: &crate::compiler::msvc::CompiledResultsStream) {
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

    let mut pdb: Option<(std::ffi::OsString, i64, bytes::Bytes)> = None;
    let mut idb: Option<(std::ffi::OsString, i64, bytes::Bytes)> = None;

    if result.exists() {
        match std::fs::File::open(&result) {
            Ok(file) => {
                let mut contents = Vec::new();
                let mut file = std::io::BufReader::new(file);
                let _ = file.read_to_end(&mut contents).unwrap();
                let origin = repair_original_path(&solution_name, &origin_working_dir, &result);        
                pdb = Some((origin, -1, bytes::Bytes::from(contents)));
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
                idb = Some((origin, -1, bytes::Bytes::from(contents)));
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

            out_err_stream.stdout.try_send(compiled_results).unwrap_or_else(|err| {
                log::warn!("try send compiled pdb results to out stream failed: {:?}", err);
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
    let (files, warning):(Vec<_>, Vec<_>) = lines.into_iter().partition(|item| item.ends_with(".i") || item.ends_with(".cpp") || item.ends_with(".c") || item.ends_with(".cc") || item.ends_with(".cxx"));
    return (files, warning);
}

fn filter_compiler_error(lines: Vec<&str>) -> (Vec<&str>, Vec<&str>) {
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
pub enum ProgramDataBase {
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

    if compiler_input.build_and_compiler_type.to_string_lossy().contains("msbuild")
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("cmake") 
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("dist") {
        
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
            else if command.to_lowercase().ends_with(".i") || command.to_lowercase().ends_with(".cpp") || command.to_lowercase().ends_with(".c") || command.to_lowercase().ends_with(".cc") || command.to_lowercase().ends_with(".cxx") {
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
                            compiler_commands: &Vec<std::ffi::OsString>, envs: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>, out_err_stream: &OutAndErrStream) -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {

    let line: String = compiler_commands.clone().into_iter()
        .map(|item| {
            return format!("{} ", item.to_string_lossy());
        })
        .collect();

    let line =  format!(r#""{}" {}"#, compiler_path.to_string_lossy(), line);

    let (status, stdout, stderr) = crate::detours::redirect::msvc_detours(solution.clone().into_string().unwrap(), project.clone().into_string().unwrap(), compiler_path.clone().into_string().unwrap(), 
                    line, working_dir.clone().into_string().unwrap(), envs.clone(), out_err_stream);

    return (status, stdout, stderr);
}

fn start_local_compiler(solution: &std::ffi::OsString, project: &std::ffi::OsString, compiler_path: &std::ffi::OsString, working_dir: &std::ffi::OsString, 
                compiler_commands: &Vec<std::ffi::OsString>, envs: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>, out_err_stream: &OutAndErrStream) 
                    -> (u32, std::sync::Arc<Vec<u8>>, std::sync::Arc<Vec<u8>>) {

    log::trace!("solution: {:?}", solution);
    log::trace!("project: {:?}", project);
    log::trace!("working dir: {:?}", working_dir);
    log::trace!("compiler path: {:?}", compiler_path);
    log::trace!("compile content: {:?}", compiler_commands);

    let start = std::time::Instant::now();
    
    let (status, stdout, stderr) = start_local_compiler_with_inject(solution, project, compiler_path, working_dir, compiler_commands, envs, out_err_stream);
    
    let elapsed = start.elapsed();
    log::info!("compile file with inject elapsed time: {:0x?}.", elapsed);
    return (status, stdout, stderr);
}

pub fn redirect_stdout_log() {
    log::info!("redirect stdout log loop thread start.");

    use std::os::windows::ffi::OsStrExt;
    let os_string = std::ffi::OsString::from("\\\\.\\pipe\\redirect_stdout_log_pipe");

    let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
    wchars.push(0);

    let mut count  = 0;
    let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};
    unsafe { loop {

        let pipe = win::System::Pipes::CreateNamedPipeW(wchars.as_ptr(), win::Storage::FileSystem::PIPE_ACCESS_INBOUND,  
        win::System::Pipes::PIPE_TYPE_MESSAGE | win::System::Pipes::PIPE_READMODE_MESSAGE | win::System::Pipes::PIPE_WAIT,
        win::System::Pipes::PIPE_UNLIMITED_INSTANCES,
        0, 0, 0, std::ptr::null_mut());
        
        if !pipe.is_null() && pipe != win::Foundation::INVALID_HANDLE_VALUE {
            
            if win::Foundation::TRUE == win::System::Pipes::ConnectNamedPipe(pipe, std::ptr::null_mut()) {

                let handle = tools::ptr::HandleBox::new(pipe);
                let _ = rt.spawn_blocking(move || {
                    let mut buffer = vec![0u8; 512];
                    let mut bytes: u32 = 0;
                    let mut moredata = String::new();
                    loop {
                        let result = win::Storage::FileSystem::ReadFile(
                            handle.get().to_owned() as _,
                            buffer.as_mut_ptr() as *mut _,
                            512,
                            &mut bytes,
                            std::ptr::null_mut()
                        );
        
                        if result == win::Foundation::FALSE || bytes == 0 {
                            let error = win::Foundation::GetLastError();

                            if error == win::Foundation::ERROR_BROKEN_PIPE {
                                //The pipe has been ended.
                                break;
                            }
                            else if error == win::Foundation::ERROR_MORE_DATA {
                                //The buffer is not enough.
                                let output = String::from_utf8_lossy(&buffer[..bytes as usize]);
                                moredata.push_str(&output);
                                continue;
                            }
                            else if error == win::Foundation::ERROR_IO_PENDING {
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
                            log::info!(target: "redirect", "{}", output);
                        }
                        else {
                            moredata.push_str(&output);
                            log::info!(target: "redirect", "{}", moredata);
                            moredata.clear();
                        }
                        //tokio::io::stdout().write_all(format!("redirect: {}\n", output).as_bytes()).await.expect("Failed to write to stdout");
                    }
                    win::System::Pipes::DisconnectNamedPipe(handle.get().to_owned() as _);
                    win::Foundation::CloseHandle(handle.get().to_owned() as _);
                });
            }
            else {
                let error = win::Foundation::GetLastError();
                log::debug!("connect named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);

                if error == win::Foundation::ERROR_NO_DATA {
    
                }
                else {
        
                }
                win::Foundation::CloseHandle(pipe);
            }
        }
        else {
            let error = win::Foundation::GetLastError();
            log::error!("connect named pipe failcreate named pipe failed. error code: {}, message: {} count: {}", error, tools::utils::get_winapi_error_message(error), count);
        }
        count += 1;
    }}

}

pub fn repair_original_path(solution: &std::ffi::OsString, working_dir: &std::ffi::OsString, path: &std::path::PathBuf) -> std::ffi::OsString {

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
            let tail = path.split_off(index); 
            
            let mut map = COMPILER_VERSION_MAP_PATH_CACHE.lock().unwrap();
            if let Some(value) = map.get(&tail) {
                return Some(value.clone());
            }
            else {
                let path = std::path::PathBuf::from(format!(r#"{}\{}"#, tools::utils::access_replica_dir(), tail));
                if std::fs::exists(&path).unwrap() {
                    map.insert(tail, path.clone().into_os_string());
                    return Some(path.into_os_string());
                }
                else {
                    return None;
                }
            }
        },
        None => {
            if compiler.to_string_lossy().contains("clang-cl") {
                return Some(compiler);
            }
            else {
                log::warn!("redirect compiler path not found MSVC not find clang-cl, so return None.");
                return None;
            }
        },
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

    use crew::compiler;

    use crate::communicate::unpackager::NAMEDPIPE_TO_GRPC_CHANNEL;

    use super::*;
    #[test]
    fn compile_sourcefile_with_inject_test() {
        println!("run msvc .c file compile test with inject");
        tools::logger::init_once_logger();
        
        std::thread::spawn(||{
            redirect_stdout_log();
        });

        crate::communicate::syscallredirectpipe::compiler_redirect_syscall();

        let rt = {crate::common::COCREW_RUNTIME.lock().unwrap().handle().clone()};

        rt.spawn(async move {
            let channel = { NAMEDPIPE_TO_GRPC_CHANNEL.namedpipe_to_grpc_rx.lock().await.take() };

            if let Some(mut rx) = channel {
                while let Some(syscall) = rx.recv().await {
                    log::info!("recv syscall: {:?}", syscall);

                    let responder = crate::communicate::syscallredirectpipe::GRPC_TO_NAMEDPIPE_CHANNEL.grpc_to_namedpipe_tx.as_ref();
                    
                    let command_result = crate::communicate::syscallredirectpipe::MirrorSysCall {
                        cid: syscall.cid,
                        api: syscall.api.clone(),
                        args: std::collections::HashMap::from([("fileinformation".to_string(), "lz4.c".to_string())]),
                    };

                    match responder.send(command_result) {
                        Ok(_) => {
                            log::info!("transmit redirect handle send callback success.");
                        },
                        Err(err) => {
                            log::error!("transmit redirect handle send callback failed: {:?}", err);
                        },
                    };
                }
            }
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
            compiler_commands.push(std::ffi::OsString::from(format!(r#"/I "{}""#, sdk_include.to_str().unwrap())));
        }
        
        for msvc_includes_path in win_compile_env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from(format!(r#"/I "{}""#, msvc_includes_path.to_str().unwrap())));
        }

        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));

        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, working_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, working_dir.to_string_lossy())));

        let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    

        let task = std::thread::spawn(move || {
            while let Ok(data) = out_receiver.try_recv() {
                println!("stream stdout: {:?}", String::from_utf8_lossy(&data));
            }

            while let Ok(data) = err_receiver.try_recv() {
                println!("stream stderr: {:?}", String::from_utf8_lossy(&data));
            }
        });

        let out_err_stream = crate::compiler::msvc::OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let envs = std::collections::HashMap::new();
        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), &std::ffi::OsString::new(),
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands, &envs, &out_err_stream);
        
        drop(out_err_stream);

        task.join().unwrap();

        println!("compile stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile stderr: {}", String::from_utf8_lossy(&stderr));
        assert!(status == 0);
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

        let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    
        let out_err_stream = crate::compiler::msvc::OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), &std::ffi::OsString::new(), 
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands, &std::collections::HashMap::new(), &out_err_stream);
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
        
        let (out_sender, out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (err_sender, err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    
        let stdout_err_stream = OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::from("GammaRayTool"), &std::ffi::OsString::new(),
            &complier_path.into_os_string(), &working_dir, &compiler_commands, &std::collections::HashMap::new(), &stdout_err_stream);
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
        
        let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    
        let stdout_err_stream = OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::from("llvm-project"), &std::ffi::OsString::from(""), 
            &complier_path.into_os_string(), &working_dir, &compiler_commands, &std::collections::HashMap::new(), &stdout_err_stream);
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

  
                if win::System::Pipes::WaitNamedPipeW(name.as_ptr(), 500) == win::Foundation::TRUE {
                   println!("wait named pipe success");
                }
                else {
                   println!("wait named pipe failed");
                }

    
        if true {
            let pipe = win::Storage::FileSystem::CreateFileW(name.as_ptr(), 
                win::Foundation::GENERIC_WRITE, 
                0,
                std::ptr::null_mut(), 
                win::Storage::FileSystem::OPEN_EXISTING, 
                win::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL, 
                std::ptr::null_mut()
            );
     
            if !pipe.is_null() && pipe != win::Foundation::INVALID_HANDLE_VALUE {
                let handle = tools::ptr::HandleBox::new(pipe);

                for i in 0..20 {
                    let message = String::from(format!("test pipe {} thread: {:?} ...", i, id));
                    let mut bytes: u32 = 0;
                    let mut overlapped: win::System::IO::OVERLAPPED = std::mem::zeroed();
                    let result = win::Storage::FileSystem::WriteFile(
                        handle.get().to_owned() as _,
                        message.as_bytes().as_ptr(),
                        message.len() as u32,
                        &mut bytes,
                        &mut overlapped
                    );
    
                    if result == win::Foundation::FALSE || bytes == 0 {
                        let error = win::Foundation::GetLastError();
                        if error == win::Foundation::ERROR_BROKEN_PIPE {
                            break;
                        }
                        println!("write pipe error, failed code: {} message: {}", error, tools::utils::get_winapi_error_message(error));
                        break;
                    }
                };

                win::Storage::FileSystem::FlushFileBuffers(handle.get().to_owned() as _);
                win::Foundation::CloseHandle(handle.get().to_owned() as _); 
            }
            else {
                let error = win::Foundation::GetLastError();
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

    #[test]
    fn clang_compile_sourcefile_with_inject_test() {
        println!("run clang-cl .c file compile test with inject");
        tools::logger::init_once_logger();
        
        std::thread::spawn(||{
            redirect_stdout_log();
        });

        let win_compile_env = crew::platform::windows::WindowsCompilerEnv::default();
        let compiler_path = std::path::PathBuf::from("G:\\Chromium\\chromium\\src\\out\\Default\\..\\..\\third_party\\llvm-build\\Release+Asserts\\bin\\clang-cl.exe");
        let working_dir = std::ffi::OsString::from("D:\\turbobuild\\draft");

        println!("draft dir: {}", working_dir.to_string_lossy());

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();

        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/nologo"));
        compiler_commands.push(std::ffi::OsString::from("/EHs"));
        compiler_commands.push(std::ffi::OsString::from("/MD"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
        compiler_commands.push(std::ffi::OsString::from("/Gy"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/GR"));
        compiler_commands.push(std::ffi::OsString::from("--warning-suppression-mappings=../../build/config/warning_suppression.txt"));

        for sdk_include in win_compile_env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {:#?}"#, sdk_include)));
        }
        
        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {:#?}"#, win_compile_env.msvc_includes_path)));
        compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));

        compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, working_dir.to_string_lossy())));
        compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, working_dir.to_string_lossy())));

        let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    
        let task = std::thread::spawn(move || {
            while let Ok(data) = out_receiver.try_recv() {
                println!("stream stdout: {:?}", String::from_utf8_lossy(&data));
            }

            while let Ok(data) = err_receiver.try_recv() {
                println!("stream stderr: {:?}", String::from_utf8_lossy(&data));
            }
        });

        let out_err_stream = crate::compiler::msvc::OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::new(), &std::ffi::OsString::new(),
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands, &std::collections::HashMap::new(), &out_err_stream);
        
        drop(out_err_stream);

        task.join().unwrap();
        assert!(status == 0);
        println!("compile stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile stderr: {}", String::from_utf8_lossy(&stderr));
    }

    #[test]
    fn compile_sourcefile_with_inject_not_find_includefiles_test() {
        println!("run msvc .c file compile test with inject not find includesfiles test");
        tools::logger::init_once_logger();
        
        std::thread::spawn(||{  
            redirect_stdout_log();
        });
        
        let win_compile_env = crew::platform::windows::WindowsCompilerEnv::default();
        let mut compiler_path = std::path::PathBuf::from(win_compile_env.compiler_path);
        compiler_path = compiler_path.join("Hostx64/x64/cl.exe");
        //"E:\TestFuture\ZLMediaKit\build\3rdpart\"
        let working_dir = std::ffi::OsString::from(r"d:\turbobuild\target\debug\Replica\Project\ZLMediaKit\build\3rdpart\");

        let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
        
        compiler_commands.push(std::ffi::OsString::from("/c"));
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from("E:\\TestFuture\\ZLMediaKit\\build"));
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from("E:\\TestFuture\\ZLMediaKit\\3rdpart"));
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from("E:\\TestFuture\\ZLMediaKit\\3rdpart\\media-server\\libmov\\include"));
        compiler_commands.push(std::ffi::OsString::from("/I"));
        compiler_commands.push(std::ffi::OsString::from("E:\\TestFuture\\ZLMediaKit\\3rdpart\\wepoll"));
        compiler_commands.push(std::ffi::OsString::from("/Zi"));
        compiler_commands.push(std::ffi::OsString::from("/W3"));
        compiler_commands.push(std::ffi::OsString::from("/WX-"));
        compiler_commands.push(std::ffi::OsString::from("/diagnostics:column"));
        compiler_commands.push(std::ffi::OsString::from("/Od"));
        compiler_commands.push(std::ffi::OsString::from("/Ob0"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("_MBCS"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("WIN32"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("_WINDOWS"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("DBUG_OFF"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("SOCKET_DEFAULT_BUF_SIZE=262144"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("HAS_EPOLL"));
        compiler_commands.push(std::ffi::OsString::from("/D"));
        compiler_commands.push(std::ffi::OsString::from("CMAKE_INTDIR=\\\"Debug\\\""));
        compiler_commands.push(std::ffi::OsString::from("/Gm-"));
        compiler_commands.push(std::ffi::OsString::from("/EHsc"));
        compiler_commands.push(std::ffi::OsString::from("/RTC1"));
        compiler_commands.push(std::ffi::OsString::from("/MTd"));
        compiler_commands.push(std::ffi::OsString::from("/GS"));
        compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
        compiler_commands.push(std::ffi::OsString::from("/Zc:inline"));
        compiler_commands.push(std::ffi::OsString::from("/Fomov.dir\\Debug\\"));
        compiler_commands.push(std::ffi::OsString::from("/FdE:\\TestFuture\\ZLMediaKit\\release\\windows\\Debug\\Debug\\mov.pdb"));
        compiler_commands.push(std::ffi::OsString::from("/external:W3"));
        compiler_commands.push(std::ffi::OsString::from("/Gd"));
        compiler_commands.push(std::ffi::OsString::from("/TC"));
        compiler_commands.push(std::ffi::OsString::from("/wd4566"));
        compiler_commands.push(std::ffi::OsString::from("/wd4819"));
        compiler_commands.push(std::ffi::OsString::from("/errorReport:prompt"));
        compiler_commands.push(std::ffi::OsString::from("/utf-8"));

        for sdk_include in win_compile_env.winkits_includes_path {
            compiler_commands.push(std::ffi::OsString::from(format!(r#"/I "{}""#, sdk_include.to_str().unwrap())));
        }
        
        for msvc_includes_path in win_compile_env.msvc_includes_path {
            compiler_commands.push(std::ffi::OsString::from(format!(r#"/I "{}""#, msvc_includes_path.to_str().unwrap())));
        }

        //compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, working_dir.to_string_lossy())));

        compiler_commands.push(std::ffi::OsString::from("E:\\TestFuture\\ZLMediaKit\\3rdpart\\media-server\\libmov\\source\\mov-udta.c"));

        let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    
        let task = std::thread::spawn(move || {
            while let Ok(data) = out_receiver.try_recv() {
                println!("stream stdout: {:?}", String::from_utf8_lossy(&data));
            }

            while let Ok(data) = err_receiver.try_recv() {
                println!("stream stderr: {:?}", String::from_utf8_lossy(&data));
            }
        });

        let out_err_stream = crate::compiler::msvc::OutAndErrStream {
            stdout: out_sender,
            stderr: err_sender,
        };

        let envs = std::collections::HashMap::new();
        //"ZLMediaKit"
        //"mov"
        let (status, stdout, stderr) = start_local_compiler(&std::ffi::OsString::from("ZLMediaKit"), &std::ffi::OsString::from("mov"),
            &compiler_path.as_os_str().to_os_string(), &working_dir, &compiler_commands, &envs, &out_err_stream);
        
        drop(out_err_stream);

        task.join().unwrap();
        assert!(status == 0);
        println!("compile stdout: {}", String::from_utf8_lossy(&stdout));
        println!("compile stderr: {}", String::from_utf8_lossy(&stderr));
    }
     
    #[test]
    fn redirect_logger_test() {
        tools::logger::init_once_logger();
        log::info!(target: "redirect", "{}", "test output");
    }

    #[test]
    fn redirect_compiler_path_test() {
        
        let current = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let path = std::path::PathBuf::from(current);
        let dir = path.parent().unwrap().join("target").join("debug");

        let path = std::ffi::OsString::from(dir.join("Replica\\MSVC\\14.33.31629\\bin\\Hostx64\\x64\\cl.exe"));
        let now = std::time::Instant::now();
        let val = redirect_compiler_path(path);
        let elapsed = now.elapsed();
        println!("redirect compiler path first elapsed: {:?} path: {:?}", elapsed, val);

        let path = std::ffi::OsString::from(dir.join("Replica\\MSVC\\14.33.31629\\bin\\Hostx64\\x64\\cl.exe"));
        let now = std::time::Instant::now();
        let val = redirect_compiler_path(path);
        let elapsed = now.elapsed();
        println!("redirect compiler path second elapsed: {:?} path: {:?}", elapsed, val);

    }
}