
mod detours;
mod hook;
mod utils;
mod replace;
mod functions;
mod logger;
pub mod syscallredirect;
use windows_sys::Win32 as win;

//TODO The current size of the package is 1.24M


struct StackWriter {
    buf: [u8; 512],
    cursor: usize,
}

impl StackWriter {
    fn new() -> Self { Self { buf: [0; 512], cursor: 0 } }    
    fn as_ptr(&self) -> *const u8 { self.buf.as_ptr() }
}

impl core::fmt::Write for StackWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len() - self.cursor - 1;
        let len = bytes.len().min(remaining);
        
        if len > 0 {
            self.buf[self.cursor..self.cursor+len].copy_from_slice(&bytes[..len]);
            self.cursor += len;
        }
        self.buf[self.cursor] = 0;
        Ok(())
    }
}

unsafe extern "system" fn custom_exception_handler(
    exception_info: *mut win::System::Diagnostics::Debug::EXCEPTION_POINTERS
) -> i32 {
    if exception_info.is_null() {
        return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }
    
    let exception_record = (*exception_info).ExceptionRecord;
    if exception_record.is_null() {
        return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }

    let code = (*exception_record).ExceptionCode as u32;
    
    if code == 0x406D1388 { //SetThreadName exception
        return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }

    let mut writer = StackWriter::new();
    use core::fmt::Write;

    if code == 0xC0000005u32 { // EXCEPTION_ACCESS_VIOLATION
        let address = (*exception_record).ExceptionAddress;
        let flags = (*exception_record).ExceptionFlags as u32;
        let num_params = (*exception_record).NumberParameters;
        let mut operation = "Unknown";
        let mut fault_address = 0usize;
        
        if (*exception_record).NumberParameters >= 2 {
            let op_code = *(*exception_record).ExceptionInformation.as_ptr();
            fault_address = *(*exception_record).ExceptionInformation.as_ptr().add(1);
            operation = if op_code == 0 { "Read" } else if op_code == 1 { "Write" } else { "Execute" };
        }

        let _ = write!(writer, "[CRASH] Access Violation (0xC0000005): {} at {:#X}, addr: {:p}, flags={:#X}, params={}\n\0", 
            operation, fault_address, address, flags, num_params);
            
        win::System::Diagnostics::Debug::OutputDebugStringA(writer.as_ptr());
        
        return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }
    else if code == 0x80000003u32 { // EXCEPTION_BREAKPOINT
        return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }
    else if code == 0x80000004u32 { // EXCEPTION_SINGLE_STEP
        return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }
    else if (code & 0xF0000000) >= 0xC0000000 {
        let _ = write!(writer, "[EXCEPTION] Code: {:#X} at {:p}\n\0", code, (*exception_record).ExceptionAddress);
        win::System::Diagnostics::Debug::OutputDebugStringA(writer.as_ptr());
    }

    return win::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
}

struct Channel {
    tx: tokio::sync::mpsc::Sender<String>, 
    rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<String>>>,
}

static LOGGER: std::sync::LazyLock<Channel> = std::sync::LazyLock::new(|| {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(1024);
        let channel = Channel { tx, rx: std::sync::Mutex::new(Some(rx)) };
        return channel; 
});

static RUNTIME: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex::<tokio::runtime::Runtime>>> = std::sync::LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name_fn(|| {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            format!("redirect-worker-{}", id)
        }).build().unwrap();
    std::sync::Arc::new(std::sync::Mutex::new(runtime))
});

/* 
static  PROJECTNAME: std::sync::LazyLock<std::sync::Mutex<Option<String>>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(None)
});
*/

//from buildassist read command line up to the present moment elapsed time is about 1.5s.
static SOLUTIONNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static PROJECTNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/* 
static  REPLICADIR: std::sync::LazyLock<std::sync::Mutex<Option<String>>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(None)
});
*/

static REPLICADIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static GENERATEDDIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static REPLICA_PDBPATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static INCLUDES: std::sync::OnceLock<Vec<std::path::PathBuf>> = std::sync::OnceLock::new();
static SOURCES: std::sync::OnceLock<std::collections::HashSet<std::ffi::OsString>> = std::sync::OnceLock::new();

static WORKINGDIR: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    if let Ok(path) = std::env::current_dir() {
        path.to_string_lossy().to_string()
    } else {
        String::from(".")
    }
});

static PROCESS_ID: std::sync::LazyLock<u32> = std::sync::LazyLock::new(|| {
    std::process::id()
});

fn read_project_property_from_stdin() {
    log!(trace, "read_project_property_from_stdin started.");
    let runtime = RUNTIME.lock().unwrap();
    runtime.spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::time::{timeout, Duration};

        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        loop {
            match timeout(Duration::from_millis(100), lines.next_line()).await {
                Ok(Ok(Some(line))) => {
                    log!(trace, "read stdin pipe to string: {:?}", line);
                    let arg = line;
                    if arg.starts_with("solution") {
                        let (_, sln) = arg.split_at("solution".len() + 1);
                        SOLUTIONNAME.set(String::from(&sln[0..sln.len()])).unwrap();
                    }
                    else if arg.starts_with("project") {
                        let (_, proj) = arg.split_at("project".len() + 1);
                        PROJECTNAME.set(String::from(&proj[0..proj.len()])).unwrap();
                    }
                    else if arg.starts_with("replica") {
                        let (_, dir) = arg.split_at("replica".len() + 1);
                        REPLICADIR.set(String::from(&dir[0..dir.len()])).unwrap();
                    }
                }
                Ok(Ok(None)) => {
                    // EOF
                    break;
                }
                Ok(Err(e)) => {
                    log!(error, "Error reading line: {:?}", e);
                    break;
                }
                Err(_) => {
                    // Timeout, no input
                    break;
                }
            }
        }
    });
    log!(trace, "read_project_property_from_stdin completed.");
}

static MODULE_PATH: std::sync::OnceLock<std::ffi::CString> = std::sync::OnceLock::new();

fn fetch_module_path(hinst: win::Foundation::HINSTANCE) {
    let mut buffer = vec![0u16; 512];
    let length = unsafe { win::System::LibraryLoader::GetModuleFileNameW(hinst, buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 {
        log!(error, "GetModuleFileNameW failed, error code: {}", unsafe {win::Foundation::GetLastError()});
    }
    else {
        use std::os::windows::ffi::OsStringExt;
        buffer.truncate(length as usize);
        let path = std::ffi::OsString::from_wide(&buffer);
        let path = std::ffi::CString::new(path.to_string_lossy().to_string()).unwrap();
        log!(debug, "current module path: {:?}", path);
        MODULE_PATH.set(path).unwrap();
    }
}

fn fetch_args_from_command() {
    let commandline = std::env::args_os();
    let commands: Vec<std::ffi::OsString> = commandline.collect();
    log!(debug, "current command line arguments: {:?}", commands);
    let mut includes = Vec::new();
    let mut sources_dir = std::collections::HashSet::new();
    let mut sources = std::collections::HashSet::new();
    let mut pdb_sub_dir = String::new();

    for (index, item) in commands.iter().enumerate() {

        let item = item.to_string_lossy();
        if item.eq("mspdbsrv.exe") {
            // Handle mspdbsrv.exe specific logic
            break;
        }
        else if item.eq("/I") || item.eq("/external:I") {
            if index + 1 < commands.len() {
                let include = commands[index + 1].to_string_lossy().to_string();
                let new = tools::utils::normalize_lexical(&include);
                includes.push(new);
            }
        }
        else if item.starts_with("/Fo") {
            if item.ends_with(".obj") {
                std::path::Path::new(&item[3..]).parent().map(|parent| {
                    let path = parent.to_string_lossy().to_string();
                    if let Some(project) = crate::SOLUTIONNAME.get() {
                        if let Some(index) = path.find(project) {
                            if index + project.len() + 1 < path.len() {
                                let (_, last) = path.split_at(index + project.len() + 1);
                                GENERATEDDIR.set(last.to_string()).unwrap();
                            }
                        }
                        else {
                            GENERATEDDIR.set(path).unwrap();    
                        }
                    }
                    else {
                        GENERATEDDIR.set(path).unwrap();
                    }
                });
            }
            else {
                GENERATEDDIR.set(item[3..].to_string()).unwrap();
            }
        }
        else if item.starts_with("/Fd") {

            let path = &item[3..];
            if std::path::PathBuf::from(path).is_absolute() {
                if let Some(solution) = crate::SOLUTIONNAME.get() {
                    if let Some(index) = path.find(solution) {
                        if index + solution.len() + 1 < path.len() {
                            let (_, last) = path.split_at(index + solution.len() + 1);
                            pdb_sub_dir = last.to_string();

                            if item.ends_with(".pdb") {
                                let modified = std::path::Path::new(&crate::REPLICADIR.get().unwrap()).join("Project").join(crate::SOLUTIONNAME.get().unwrap()).join(&pdb_sub_dir);
                                REPLICA_PDBPATH.set(modified.to_string_lossy().to_string()).unwrap();
                            }
                            else {
                                let modified = std::path::Path::new(&crate::REPLICADIR.get().unwrap()).join("Project").join(crate::SOLUTIONNAME.get().unwrap()).join(&pdb_sub_dir).join("vc143.pdb");
                                REPLICA_PDBPATH.set(modified.to_string_lossy().to_string()).unwrap();
                            }

                            std::path::Path::new(last).parent().map(|parent| {
                                GENERATEDDIR.get_or_init(|| parent.to_string_lossy().to_string());
                            });
                        }
                    }
                }
            }
            else {
                pdb_sub_dir = path.to_string();
                if item.ends_with(".pdb") {
                    let modified = std::path::Path::new(&crate::WORKINGDIR.as_str()).join(&pdb_sub_dir);
                    REPLICA_PDBPATH.set(modified.to_string_lossy().to_string()).unwrap();
                }
                else {
                    let modified = std::path::Path::new(&crate::WORKINGDIR.as_str()).join(&pdb_sub_dir).join("vc143.pdb");
                    REPLICA_PDBPATH.set(modified.to_string_lossy().to_string()).unwrap();
                }
            }
        }
        else if item.ends_with(".c") || item.ends_with(".cpp") || item.ends_with(".cc") || item.ends_with(".cxx") {
            let path = std::path::Path::new(item.as_ref());
            if let Some(dir) = path.parent() {
                sources_dir.insert(dir.to_path_buf());
            };
            let newpath = path.with_extension("");
            sources.insert(newpath.as_os_str().to_owned());
        }
    }
    if !sources.is_empty() {
        SOURCES.set(sources).unwrap();
    }

    if pdb_sub_dir.is_empty() {

        if let Some(replica) = crate::REPLICADIR.get() {
            let modified = std::path::Path::new(&replica).join("Project").join(crate::SOLUTIONNAME.get().unwrap()).join("vc143.pdb");
            REPLICA_PDBPATH.set(modified.to_string_lossy().to_string()).unwrap();
        }
    }

    log!(debug, "SOLUTIONNAME: {:?}", SOLUTIONNAME.get());
    log!(debug, "PROJECTNAME: {:?}", PROJECTNAME.get());
    log!(debug, "GENERATEDDIR: {:?}", GENERATEDDIR);
    log!(debug, "REPLICA_PDBPATH: {:?}", REPLICA_PDBPATH);
    log!(debug, "WORKINGDIR: {:?}", WORKINGDIR.as_str());
    log!(debug, "REPLICADIR: {:?}", REPLICADIR.get());
    log!(debug, "SOURCES: {:?}", SOURCES.get());

    includes.extend(sources_dir.iter().cloned());

    for (key, value) in std::env::vars() {
        if key.to_lowercase() == "include" {
            let paths: Vec<&str> = value.split(';').collect();
            for path in paths {
                if !path.is_empty() {
                    includes.push(std::path::PathBuf::from(path));
                }
            }
        }
    }
    INCLUDES.set(includes).unwrap();
    log!(debug, "INCLUDES: {:?}", INCLUDES.get());

}   

fn uninit_custom_resource() {
    log!(info, "[{:?}] uninit custom resource. receive is closed: {}", crate::PROJECTNAME.get(), LOGGER.tx.capacity());
}

unsafe fn show_message_box_for_debug() {
    use std::os::windows::ffi::OsStrExt;
    let text: Vec<u16> = std::ffi::OsStr::new("Debug BreakPoint")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let caption: Vec<u16> = std::ffi::OsStr::new("Attach Programe")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    log!(info, "show debug message box for process attach");
    
    //just for attach debug, when message box block process run.
    win::UI::WindowsAndMessaging::MessageBoxW(0 as win::Foundation::HWND, text.as_ptr(), caption.as_ptr(), 0);
}

thread_local! {
    static IN_HOOK: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[no_mangle]
unsafe extern "system" fn DllMain(
    hinst: win::Foundation::HINSTANCE, 
    fdw_reason: u32, 
    _reserved: *mut core::ffi::c_void
) -> i32 {
    
    if crate::detours::DetourIsHelperProcess() == win::Foundation::TRUE {
        //println!("target application is a helper process, so do nothing.");
        return win::Foundation::TRUE;
    }

    match fdw_reason {
        win::System::SystemServices::DLL_PROCESS_ATTACH => {

            //win::System::Diagnostics::Debug::AddVectoredExceptionHandler(1, Some(custom_exception_handler));
            //force_unbuffered_output();
            //show_message_box_for_debug();
            
            win::System::LibraryLoader::DisableThreadLibraryCalls(hinst);

            fetch_module_path(hinst);
            read_project_property_from_stdin();

            fetch_args_from_command(); 

            crate::logger::redirect_stdout_log_2_cocrew();
            
            crate::syscallredirect::async_connect_syscall_namedpipe();
            
            let ret = crate::detours::DetourRestoreAfterWith();
            if ret == win::Foundation::FALSE {
                let error_code = win::Foundation::GetLastError();
                log!(error, "DetourRestoreAfterWith failed, error code: {}.", error_code);
            }

            let ret = crate::detours::DetourTransactionBegin();
            if ret != win::Foundation::NO_ERROR as i32 {
                let error_code = win::Foundation::GetLastError();
                log!(error, "DetourTransactionBegin failed, error code: {}.", error_code);
            }

            let ret = crate::detours::DetourUpdateThread(win::System::Threading::GetCurrentThread() as _);
            if ret != win::Foundation::NO_ERROR as i32 {
                let error_code = win::Foundation::GetLastError();
                log!(error, "DetourUpdateThread failed, erro code: {}.", error_code);
            }
        
            //ENTRYPOINT = crate::detours::DetourGetEntryPoint(std::ptr::null_mut());
            //crate::detours::DetourAttach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);
            //hook entry point

            crate::hook::init_hook();
            
            let ret = crate::detours::DetourTransactionCommit();
            if ret != win::Foundation::NO_ERROR as i32 {
                let error_code = win::Foundation::GetLastError();
                log!(error, "DetourTransactionCommit failed, error code: {}.", error_code);
            }
        },
        win::System::SystemServices::DLL_THREAD_ATTACH => {

        },
        win::System::SystemServices::DLL_PROCESS_DETACH => {
            crate::detours::DetourTransactionBegin();
            crate::detours::DetourUpdateThread(win::System::Threading::GetCurrentThread() as _);
           
            //DetourDetach();

            //crate::detours::DetourDetach(core::ptr::addr_of_mut!(ENTRYPOINT), main as *mut _);
            //unhook entry point
            crate::detours::DetourTransactionCommit();

            uninit_custom_resource();
        }
        win::System::SystemServices::DLL_THREAD_DETACH => {

        },
        _ => {
        },
    }

    return win::Foundation::TRUE;
}


extern "C" {
    fn setvbuf(stream: *mut std::ffi::c_void, buffer: *mut std::ffi::c_char, mode: std::ffi::c_int, size: usize) -> std::ffi::c_int;
    fn __acrt_iob_func(index: std::ffi::c_uint) -> *mut std::ffi::c_void;
    fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
}


const _IOFBF: std::ffi::c_int = 0; 
const _IOLBF: std::ffi::c_int = 1; 
const _IONBF: std::ffi::c_int = 2; 


unsafe fn force_unbuffered_output() {
    
    let stdout_ptr = __acrt_iob_func(1);
    let stderr_ptr = __acrt_iob_func(2);
    
    if !stdout_ptr.is_null() && !stderr_ptr.is_null() {

        let _result_stdout = setvbuf(stdout_ptr, std::ptr::null_mut(), _IOLBF, 0);
        let _result_stdout = setvbuf(stderr_ptr, std::ptr::null_mut(), _IOLBF, 0);
        

        // let result1 = setvbuf(stdout_ptr, std::ptr::null_mut(), _IONBF, 0);
        // let result2 = setvbuf(stderr_ptr, std::ptr::null_mut(), _IONBF, 0);


        fflush(stdout_ptr);
        fflush(stderr_ptr);
    } else {
        crate::log!(warn, "Failed to get stdout/stderr pointers");
    }
}