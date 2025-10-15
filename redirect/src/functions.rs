
use winapi::{
    shared::{minwindef::{DWORD, LPVOID}, ntdef::{LPCWSTR, LPWSTR}},
    um::{minwinbase::LPSECURITY_ATTRIBUTES, winnt::{HANDLE, LPCSTR, LPSTR, WCHAR}}
};

pub static mut CREATE_FILE_A: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_A_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_QUERY_DIRECTORY_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_CREATE_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_PROCESS_A_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_PROCESS_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;

static SYS_CALL_ID: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<u32>>> = std::sync::LazyLock::new(|| {
    let pid = std::process::id();
    std::sync::Arc::new(std::sync::Mutex::new(pid * 10000))
});

static INCLUDES_CACHE: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>> = std::sync::LazyLock::new(|| {
    std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))
});


const PIPE_PREFIX_CONTENT_W: &[u16] = &['\\' as u16, '\\' as u16, '.' as u16, '\\' as u16, 'p' as u16, 'i' as u16, 'p' as u16, 'e' as u16, '\\' as u16];  //\\.\\pipe\\ or \\??\\pipe\\

pub unsafe fn start_with_pipe_w(lp_file_name: *const u16) -> bool {
    if lp_file_name.is_null() || lp_file_name == 0 as *const u16{
        return false;
    } 
    else {
        let len = PIPE_PREFIX_CONTENT_W.len() as isize;
        for i in 0..len {
            let uchar = *lp_file_name.offset(i as isize);
            if uchar != PIPE_PREFIX_CONTENT_W[i as usize] {
                return false;
            }
        }
        return true;
    }
}

pub unsafe fn create_file_a(
    lp_file_name: LPCSTR,
    dw_desired_access: DWORD,
    dw_share_mode: DWORD,
    lp_security_attributes: LPSECURITY_ATTRIBUTES,
    dw_creation_disposition: DWORD,
    dw_flags_and_attributes: DWORD,
    h_template_file: HANDLE,
) -> HANDLE {

    let path = crate::utils::convert::lpstr_2_string(lp_file_name);
  
    if let Ok(mut path) = path {
        crate::log!(trace, "create_file_a hook path: {}", path);

        let create_file_a: extern "system" fn(
            lp_file_name: LPCSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_A);

        let replace = crate::replace::replace(&mut path);
        if replace == crate::replace::ReplaceResult::Success{
            crate::log!(trace, "create_file_a replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpstr(path);

            let handle = create_file_a(
                fake_path,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            {
                let c_string = std::ffi::CString::from_raw(fake_path);
                drop(c_string);
            }

            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "create_file_a failed! error code: {}.", error_code);
            }

            return handle;
        }
        else {
            let handle = create_file_a(
                lp_file_name,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "create_file_a failed! error code: {}.", error_code)
            }

            return handle;
        }
    }
    else {
        return 0 as HANDLE;
    }
}

pub unsafe fn create_file_w(
    lp_file_name: LPCWSTR,
    dw_desired_access: DWORD,
    dw_share_mode: DWORD,
    lp_security_attributes: LPSECURITY_ATTRIBUTES,
    dw_creation_disposition: DWORD,
    dw_flags_and_attributes: DWORD,
    h_template_file: HANDLE,
) -> HANDLE {
    
    if start_with_pipe_w(lp_file_name) {
        let create_file_w_inner: extern "system" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W);

        let handle = create_file_w_inner(
            lp_file_name,
            dw_desired_access,
            dw_share_mode,
            lp_security_attributes,
            dw_creation_disposition,
            dw_flags_and_attributes,
            h_template_file,
        );
        return handle;
    }

    let path = crate::utils::convert::lpwstr_2_string(lp_file_name);
    if let Some(mut path) = path {

        crate::log!(trace, "create_file_w hook path: {}", path);

        let create_file_w: extern "system" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W);

        let replace = crate::replace::replace(&mut path);

        if replace == crate::replace::ReplaceResult::Success {
            crate::log!(trace, "create_file_w replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpwstr(path);
                
            let handle = create_file_w(
                fake_path.as_ptr() as winapi::um::winnt::LPWSTR,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }

            return handle;
        }
        else {

            let handle = create_file_w(
                lp_file_name,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }

            return handle;    
        }
    }
    else {
        return 0 as HANDLE;
    }

}

pub unsafe fn kernelbase_create_file_a(
    lp_file_name: LPCSTR,
    dw_desired_access: DWORD,
    dw_share_mode: DWORD,
    lp_security_attributes: LPSECURITY_ATTRIBUTES,
    dw_creation_disposition: DWORD,
    dw_flags_and_attributes: DWORD,
    h_template_file: HANDLE,
) -> HANDLE {

    let path = crate::utils::convert::lpstr_2_string(lp_file_name);
  
    if let Ok(mut path) = path {
        
        crate::log!(trace, "kernelbase_create_file_a hook path: {}", path);

        let create_file_a: extern "system" fn(
            lp_file_name: LPCSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_A_KERNEL_BASE);

        let replace = crate::replace::replace(&mut path);

        if replace == crate::replace::ReplaceResult::Success {
            crate::log!(trace, "kernelbase_create_file_a replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpstr(path);
            let handle = create_file_a(
                fake_path,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            {
                let c_string = std::ffi::CString::from_raw(fake_path);
                drop(c_string);
            }

            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "kernelbase create_file_a failed! error_code: {}.", error_code);
            }

            return handle;
        }
        else {

            let handle = create_file_a(
                lp_file_name,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "kernelbase create_file_a failed! error_code: {}.", error_code);
            }
            return handle;    
        }
    }
    else {
        return 0 as HANDLE;
    }
}

pub unsafe fn kernelbase_create_file_w(
    lp_file_name: LPCWSTR,
    dw_desired_access: DWORD,
    dw_share_mode: DWORD,
    lp_security_attributes: LPSECURITY_ATTRIBUTES,
    dw_creation_disposition: DWORD,
    dw_flags_and_attributes: DWORD,
    h_template_file: HANDLE,
) -> HANDLE {

    if start_with_pipe_w(lp_file_name) {
        let create_file_w_inner: extern "system" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W_KERNEL_BASE);

        let handle = create_file_w_inner(
            lp_file_name,
            dw_desired_access,
            dw_share_mode,
            lp_security_attributes,
            dw_creation_disposition,
            dw_flags_and_attributes,
            h_template_file,
        );
        return handle;
    }
    
    let option_path = crate::utils::convert::lpwstr_2_string(lp_file_name);

    if let Some(mut path) = option_path.clone() {

        if CREATE_FILE_W_KERNEL_BASE as usize == 0 {
            crate::log!(error, "can not find kernelbase create_file_w");
            return winapi::um::handleapi::INVALID_HANDLE_VALUE;
        }

        let create_file_w: extern "system" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W_KERNEL_BASE);

        crate::log!(trace, "kernelbase_create_file_w hook path: {}", path);
        let replace = crate::replace::replace(&mut path);
        if replace == crate::replace::ReplaceResult::Success {
            crate::log!(trace, "kernelbase_create_file_w replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpwstr(path);
            
            let handle = create_file_w(
                fake_path.as_ptr() as winapi::um::winnt::LPWSTR,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );
    
            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "kernelbase create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }
    
            return handle; 
        }
        else if replace == crate::replace::ReplaceResult::FilePath {
            crate::log!(trace, "kernelbase_create_file_w replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpwstr(path.clone());

            let handle = create_file_w(fake_path.as_ptr() as winapi::um::winnt::LPWSTR,
                dw_desired_access, dw_share_mode, lp_security_attributes,
                dw_creation_disposition, dw_flags_and_attributes, h_template_file,
            );
    
            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = winapi::um::errhandlingapi::GetLastError();

                if error_code == winapi::shared::winerror::ERROR_FILE_NOT_FOUND {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let mut args =  std::collections::HashMap::<String, String>::new();
                    args.insert("filename".to_string(), option_path.unwrap());
                    args.insert("expect".to_string(), path.clone());

                    let call = {
                        let mut cid = SYS_CALL_ID.lock().unwrap();
                        let call = crate::syscallredirect::MirrorSysCall {
                            id: *cid,
                            api: "CreateFileW".into(),
                            args,
                            responder: tx,
                        };
                        *cid += 1;
                        call
                    };
                    let id = call.id.clone();
                    crate::log!(trace, "kernelbase_create_file_w file id: {} path: {} process: {} thread: {:?}", id.clone(), path, std::process::id(), std::thread::current().id());

                    crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(call).unwrap();
                    let expects = rx.blocking_recv().unwrap();
                    if let Some(_) = expects.get("expect") {
                        let handle = create_file_w(fake_path.as_ptr() as winapi::um::winnt::LPWSTR, dw_desired_access, dw_share_mode, lp_security_attributes,
                            dw_creation_disposition, dw_flags_and_attributes, h_template_file,
                        );
                        return handle;
                    }
                    else {
                        return handle;
                    }   
                }
                crate::log!(error, "kernelbase create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }
            return handle;
        }
        else {
            let handle = create_file_w(
                lp_file_name,
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );
    
            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "kernelbase create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }
            return handle;   
        }
    }
    else {
        return 0 as HANDLE;
    }

}

pub unsafe fn kernelbase_create_process_a(
    lp_application_name: LPCSTR,
    lp_command_line: LPSTR,
    lp_process_attributes: LPSECURITY_ATTRIBUTES,
    lp_thread_attributes: LPSECURITY_ATTRIBUTES,
    b_inherit_handles: super::ntdef::types::BOOL,
    dw_creation_flags: DWORD,
    lp_environment: LPVOID,
    lp_current_directory: LPCSTR,
    lp_startup_info: crate::detours::LPSTARTUPINFOA,
    lp_process_information: crate::detours::LPPROCESS_INFORMATION,
) -> HANDLE {

    let application = crate::utils::convert::lpstr_2_string(lp_application_name);
  
    if let Ok(application) = application {
        
        crate::log!(info, "kernelbase_create_process_a hook path: {}", application);
        //crate::log!(info, "kernelbase_create_process_a hook commandline: {:?}", crate::utils::convert::lpstr_2_string(lp_command_line));

        let dllpath = crate::MODULE_PATH.get();

        if application.ends_with("cl.exe") && dllpath.is_some() && crate::IN_HOOK.get() == false {
            crate::IN_HOOK.set(true);

            let mut stdin_write_handle: Option<winapi::shared::ntdef::HANDLE> = None;

            if ((*lp_startup_info).dwFlags & winapi::um::winbase::STARTF_USESTDHANDLES) != 0 {

                let mut pipe_attributes = winapi::um::minwinbase::SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: std::ptr::null_mut(),
                    bInheritHandle: winapi::shared::minwindef::TRUE,
                };

                let mut h_stdin_read: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
                let mut h_stdin_write: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();

                let ret = winapi::um::namedpipeapi::CreatePipe(
                    &mut h_stdin_read as winapi::shared::ntdef::PHANDLE, 
                    &mut h_stdin_write as winapi::shared::ntdef::PHANDLE, 
                    &mut pipe_attributes, 
                    0
                );

                if winapi::shared::minwindef::FALSE == ret {
                    crate::log!(error, "create input pipe failed.")
                }
                else {
                    (*lp_startup_info).hStdInput = h_stdin_read as *mut std::ffi::c_void;
                    stdin_write_handle = Some(h_stdin_write);
                }
            }

            let ret = crate::detours::DetourCreateProcessWithDllExA(
                lp_application_name,
                lp_command_line,
                lp_process_attributes as *mut crate::detours::_SECURITY_ATTRIBUTES,
                lp_thread_attributes as *mut crate::detours::_SECURITY_ATTRIBUTES, 
                b_inherit_handles as i32, 
                dw_creation_flags,
                lp_environment as *mut std::ffi::c_void,
                lp_current_directory, 
                lp_startup_info,
                lp_process_information, 
                dllpath.unwrap().as_ptr() as *const i8,
                Option::None
            );
            
            if ret == winapi::shared::minwindef::TRUE {
                if let Some(stdin_write) = stdin_write_handle {
                    pass_project_and_replica_to_redriect(stdin_write, crate::SOLUTIONNAME.get().unwrap(), crate::PROJECTNAME.get().unwrap(), crate::REPLICADIR.get().unwrap());
                }

                return (*lp_process_information).hProcess as winapi::um::winnt::HANDLE;
            }
            else {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "detour create process withdllexa failed! error_code: {}.", error_code);

                let create_process_a: extern "system" fn(
                    lp_application_name: LPCSTR,
                    lp_command_line: LPSTR,
                    lp_process_attributes: LPSECURITY_ATTRIBUTES,
                    lp_thread_attributes: LPSECURITY_ATTRIBUTES,
                    b_inherit_handles: super::ntdef::types::BOOL,
                    dw_creation_flags: DWORD,
                    lp_environment: LPVOID,
                    lp_current_directory: LPCSTR,
                    lp_startup_info: crate::detours::LPSTARTUPINFOA,
                    lp_process_information: crate::detours::LPPROCESS_INFORMATION,
                ) -> HANDLE = std::mem::transmute(CREATE_PROCESS_A_KERNEL_BASE);
                

                let handle = create_process_a (
                    lp_application_name,
                    lp_command_line,
                    lp_process_attributes,
                    lp_thread_attributes,
                    b_inherit_handles,
                    dw_creation_flags,
                    lp_environment,
                    lp_current_directory,
                    lp_startup_info,
                    lp_process_information);
                
                if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    crate::log!(error, "kernelbase_create_process_a failed! error_code: {}.", error_code);
                }
                return handle;
            }
        }
        else
        {
            let create_process_a: extern "system" fn(
                lp_application_name: LPCSTR,
                lp_command_line: LPSTR,
                lp_process_attributes: LPSECURITY_ATTRIBUTES,
                lp_thread_attributes: LPSECURITY_ATTRIBUTES,
                b_inherit_handles: super::ntdef::types::BOOL,
                dw_creation_flags: DWORD,
                lp_environment: LPVOID,
                lp_current_directory: LPCSTR,
                lp_startup_info: crate::detours::LPSTARTUPINFOA,
                lp_process_information: crate::detours::LPPROCESS_INFORMATION,
            ) -> HANDLE = std::mem::transmute(CREATE_PROCESS_A_KERNEL_BASE);
    
            let handle = create_process_a (
                lp_application_name,
                lp_command_line,
                lp_process_attributes,
                lp_thread_attributes,
                b_inherit_handles,
                dw_creation_flags,
                lp_environment,
                lp_current_directory,
                lp_startup_info,
                lp_process_information);
            
            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "kernelbase_create_process_a failed! error_code: {}.", error_code);
            }
    
            return handle;
        }
    }
    else {
        return 0 as HANDLE;
    }
}

pub unsafe fn kernelbase_create_process_w(
    lp_application_name: LPCWSTR,
    lp_command_line: LPWSTR,
    lp_process_attributes: LPSECURITY_ATTRIBUTES,
    lp_thread_attributes: LPSECURITY_ATTRIBUTES,
    b_inherit_handles: super::ntdef::types::BOOL,
    dw_creation_flags: DWORD,
    lp_environment: LPVOID,
    lp_current_directory: LPCWSTR,
    lp_startup_info: crate::detours::LPSTARTUPINFOW,
    lp_process_information: crate::detours::LPPROCESS_INFORMATION,
) -> HANDLE {
    
    let application = crate::utils::convert::lpwstr_2_string(lp_application_name);

    if let Some(application) = application {
        
        crate::log!(info, "kernelbase_create_process_w hook path: {}", application);
        //crate::log!(info, "kernelbase_create_process_w hook commandline: {:?}", crate::utils::convert::lpwstr_2_string(lp_command_line));

        let dllpath = crate::MODULE_PATH.get();

        if (application.ends_with("cl.exe") || application.ends_with("mspdbsrv.exe")) && dllpath.is_some() && crate::IN_HOOK.get() == false {
            
            crate::IN_HOOK.set(true);
            
            let mut stdin_write_handle: Option<winapi::shared::ntdef::HANDLE> = None;

            let mut pipe_attributes = winapi::um::minwinbase::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: winapi::shared::minwindef::TRUE,
            };

            let mut h_stdin_read: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();
            let mut h_stdin_write: winapi::shared::ntdef::HANDLE = std::ptr::null_mut();

            let ret = winapi::um::namedpipeapi::CreatePipe(
                &mut h_stdin_read as winapi::shared::ntdef::PHANDLE, 
                &mut h_stdin_write as winapi::shared::ntdef::PHANDLE, 
                &mut pipe_attributes, 
                0
            );

            let mut b_inherit_handles = b_inherit_handles;
            if winapi::shared::minwindef::FALSE == ret {
                crate::log!(error, "create input pipe failed.")
            }
            else {

                if application.ends_with("mspdbsrv.exe") {

                    let current_stdout = winapi::um::processenv::GetStdHandle(winapi::um::winbase::STD_OUTPUT_HANDLE);
                    let current_stderr = winapi::um::processenv::GetStdHandle(winapi::um::winbase::STD_ERROR_HANDLE);
                    
                    if !current_stdout.is_null() {
                        winapi::um::handleapi::SetHandleInformation(
                            current_stdout,
                            winapi::um::winbase::HANDLE_FLAG_INHERIT,
                            0
                        );
                    }
                    
                    if !current_stderr.is_null() {
                        winapi::um::handleapi::SetHandleInformation(
                            current_stderr,
                            winapi::um::winbase::HANDLE_FLAG_INHERIT,
                            0
                        );
                    }

                    (*lp_startup_info).hStdOutput = std::ptr::null_mut();
                    (*lp_startup_info).hStdError = std::ptr::null_mut();
                }
                b_inherit_handles = super::ntdef::enums::TRUE;

                winapi::um::handleapi::SetHandleInformation(h_stdin_write, winapi::um::winbase::HANDLE_FLAG_INHERIT, 0);
                (*lp_startup_info).hStdInput = h_stdin_read as *mut std::ffi::c_void;
                (*lp_startup_info).dwFlags |=  winapi::um::winbase::STARTF_USESTDHANDLES;

                stdin_write_handle = Some(h_stdin_write);
            }

            let ret = crate::detours::DetourCreateProcessWithDllExW(
                lp_application_name,
                lp_command_line,
                lp_process_attributes as *mut crate::detours::_SECURITY_ATTRIBUTES,
                lp_thread_attributes as *mut crate::detours::_SECURITY_ATTRIBUTES, 
                b_inherit_handles as i32, 
                dw_creation_flags,
                lp_environment as *mut std::ffi::c_void,
                lp_current_directory, 
                lp_startup_info,
                lp_process_information,
                dllpath.unwrap().as_ptr() as *const i8,
                Option::None
            );
            
            if ret == winapi::shared::minwindef::TRUE {
                
                winapi::um::handleapi::CloseHandle(h_stdin_read);

                if let Some(stdin_write) = stdin_write_handle {
                    pass_project_and_replica_to_redriect(stdin_write, crate::SOLUTIONNAME.get().unwrap(), crate::PROJECTNAME.get().unwrap(), crate::REPLICADIR.get().unwrap());
                }

                return (*lp_process_information).hProcess as winapi::um::winnt::HANDLE;
            }
            else {
                winapi::um::handleapi::CloseHandle(h_stdin_read);
                winapi::um::handleapi::CloseHandle(h_stdin_write);

                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "detour create process withdllexw failed! error_code: {}.", error_code);

                let create_process_w: extern "system" fn(
                    lp_application_name: LPCWSTR,
                    lp_command_line: LPWSTR,
                    lp_process_attributes: LPSECURITY_ATTRIBUTES,
                    lp_thread_attributes: LPSECURITY_ATTRIBUTES,
                    b_inherit_handles: super::ntdef::types::BOOL,
                    dw_creation_flags: DWORD,
                    lp_environment: LPVOID,
                    lp_current_directory: LPCWSTR,
                    lp_startup_info: crate::detours::LPSTARTUPINFOW,
                    lp_process_information: crate::detours::LPPROCESS_INFORMATION,
                ) -> HANDLE = std::mem::transmute(CREATE_PROCESS_W_KERNEL_BASE);
                

                let handle = create_process_w (
                    lp_application_name,
                    lp_command_line,
                    lp_process_attributes,
                    lp_thread_attributes,
                    b_inherit_handles,
                    dw_creation_flags,
                    lp_environment,
                    lp_current_directory,
                    lp_startup_info,
                    lp_process_information);
                
                if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    crate::log!(error, "kernelbase create_file_w failed! error_code: {}.", error_code);
                }
                return handle;
            }
        } 
        else {
            let create_process_w: extern "system" fn(
                lp_application_name: LPCWSTR,
                lp_command_line: LPWSTR,
                lp_process_attributes: LPSECURITY_ATTRIBUTES,
                lp_thread_attributes: LPSECURITY_ATTRIBUTES,
                b_inherit_handles: super::ntdef::types::BOOL,
                dw_creation_flags: DWORD,
                lp_environment: LPVOID,
                lp_current_directory: LPCWSTR,
                lp_startup_info: crate::detours::LPSTARTUPINFOW,
                lp_process_information: crate::detours::LPPROCESS_INFORMATION,
            ) -> HANDLE = std::mem::transmute(CREATE_PROCESS_W_KERNEL_BASE);
            
            let handle = create_process_w (
                lp_application_name,
                lp_command_line,
                lp_process_attributes,
                lp_thread_attributes,
                b_inherit_handles,
                dw_creation_flags,
                lp_environment,
                lp_current_directory,
                lp_startup_info,
                lp_process_information);
            
            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let error_code = winapi::um::errhandlingapi::GetLastError();
                crate::log!(error, "kernelbase create_file_w failed! error_code: {}.", error_code);
            }
            return handle;
        }
    }
    else {
        return 0 as HANDLE;
    }
}

thread_local! {
    static NT_HANDLE_AND_FILENAMES: std::cell::RefCell<std::collections::HashMap<windows_sys::Win32::Foundation::HANDLE, Vec<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

thread_local! {
    static NT_HANDLE_AND_DIR: std::cell::RefCell<std::collections::HashMap<windows_sys::Win32::Foundation::HANDLE, String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

pub unsafe fn nt_query_directory_file(
    file_handle: windows_sys::Win32::Foundation::HANDLE,
    event: windows_sys::Win32::Foundation::HANDLE,
    apc_routine: windows_sys::Win32::System::IO::PIO_APC_ROUTINE,
    apc_context: *const core::ffi::c_void,
    io_status_block: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
    file_information: *mut core::ffi::c_void,
    length: u32,
    file_information_class: windows_sys::Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS,
    return_single_entry: bool,
    file_name: *const windows_sys::Win32::Foundation::UNICODE_STRING,
    restart_scan: bool
    ) -> windows_sys::Win32::Foundation::NTSTATUS {

    let nt_query_directory_file: extern "system" fn(
        filehandle: windows_sys::Win32::Foundation::HANDLE,
        event: windows_sys::Win32::Foundation::HANDLE,
        apcroutine:  windows_sys::Win32::System::IO::PIO_APC_ROUTINE,
        apccontext: *const core::ffi::c_void,
        iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
        fileinformation: *mut core::ffi::c_void,
        length: u32,
        fileinformationclass: windows_sys::Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS,
        returnsingleentry: bool,
        filename: *const windows_sys::Win32::Foundation::UNICODE_STRING,
        restartscan: bool,
    ) -> windows_sys::Win32::Foundation::NTSTATUS = std::mem::transmute(NT_QUERY_DIRECTORY_FILE);

    if (restart_scan && !file_name.is_null()) || (file_information_class != windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation) {
        let nt_status = nt_query_directory_file(
            file_handle,
            event,
            apc_routine,
            apc_context,
            io_status_block,
            file_information,
            length,
            file_information_class,
            return_single_entry,
            file_name,
            restart_scan
        );

        if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS && !file_information.is_null() {
            /* 
            let mut current_offset = 0usize;
            let mut entry_count = 0;
            
            loop {
                let current_entry = (file_information as *const u8).add(current_offset);
                entry_count += 1;
                match file_information_class {
                    windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation => {
                        let file_info = current_entry as *const windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;
                        let file_name_length_bytes = (*file_info).FileNameLength as usize;

                        if file_name_length_bytes > 0 {
                            let file_name_slice = std::slice::from_raw_parts((*file_info).FileName.as_ptr(), file_name_length_bytes / 2);
                            if let Ok(file_name_str) = String::from_utf16(file_name_slice) {
                                crate::log!(trace, "nt_query_directory_file single entry #{}: with file: '{}'", entry_count, file_name_str);
                            }
                        }

                        let next_entry_offset = (*file_info).NextEntryOffset;
                        if next_entry_offset == 0 {
                            break;
                        }
                        else {
                            current_offset += next_entry_offset as usize;
                        }
                    }
                    _ => {
                        crate::log!(trace, "nt_query_directory_file: unsupported file_information_class: {:?}", file_information_class);
                    }
                }

                if current_offset >= length as usize {
                    crate::log!(warn, "reached buffer end, processed {} entries", entry_count);
                    break;
                }
            }
            */
        }
        else {
            if !file_name.is_null() {
                let buffer = (*file_name).Buffer;
                let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
                crate::log!(trace, "nt_query_directory_file single file failed with status {:#X} filename: {:?} dir filehandle: {:?}", nt_status, name, file_handle);
            }

            if nt_status == windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES {
            }
            else if nt_status == windows_sys::Win32::Foundation::STATUS_BUFFER_OVERFLOW {
                crate::log!(warn, "nt_query_directory_file buffer overflow occurred, consider increasing buffer size.");
            }
            else {
                crate::log!(error, "nt_query_directory_file failed with status: {:#X}", nt_status);
            }
        }

        return nt_status;
    }
    else {
        //second or subsequent query.
        if (*io_status_block).Information < length as usize && (*io_status_block).Information > 0 && (*io_status_block).Anonymous.Status == windows_sys::Win32::Foundation::STATUS_SUCCESS {

            let maybe_filenames = NT_HANDLE_AND_FILENAMES.with(|cell| {
                let handle_and_filenames = cell.borrow();
                return handle_and_filenames.get(&file_handle).cloned();
            });

            //crate::logger::output_debug_string(&format!("nt_query_directory_file maybe_filenames: {:?} handle: {:?}", std::thread::current().id(), file_handle));

            if let Some(filenames) = maybe_filenames {
                    
                let mut entries_written = 0;
                let mut current_offset = 0usize;

                for (index, file) in filenames.iter().enumerate() {
                    let virtual_file_name: Vec<u16> = file.encode_utf16().collect();
                    let virtual_file_name_bytes: u32 = (virtual_file_name.len() * 2) as u32;

                    let base_size = std::mem::size_of::<windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION>() - std::mem::size_of::<u16>();
                    let entry_size = base_size + virtual_file_name_bytes as usize;

                    let aligned_entry_size = (entry_size + 7) & !7; // Align to 8 bytes

                    if current_offset + aligned_entry_size as usize > length as usize {
                        crate::log!(error, "nt_query_directory_file: buffer overflow, current_offset: {}, aligned_entry_size: {}, length: {}", current_offset, aligned_entry_size, length);
                        break;
                    }

                    let entry_ptr = file_information.add(current_offset);
                    std::ptr::write_bytes(entry_ptr, 0u8, aligned_entry_size);

                    let entry = entry_ptr as *mut windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;

                    let next_offset = if index == filenames.len() - 1 || return_single_entry {
                        0
                    } else {
                        aligned_entry_size as u32
                    };

                    (*entry).NextEntryOffset = next_offset;                        
                    (*entry).FileIndex = index as u32;
                    (*entry).FileNameLength = virtual_file_name_bytes as u32;
                    
                    std::ptr::copy_nonoverlapping(
                        virtual_file_name.as_ptr(),
                        (*entry).FileName.as_mut_ptr(),
                        virtual_file_name.len()
                    );

                    current_offset += aligned_entry_size as usize;
                    entries_written += 1;

                    if return_single_entry {
                        NT_HANDLE_AND_FILENAMES.with(|cell| {
                            let mut handle_and_filenames = cell.borrow_mut();
                            if index + 1 < filenames.len() {
                                handle_and_filenames.insert(file_handle, filenames[index + 1..].to_vec());
                            }
                            else {
                                handle_and_filenames.remove(&file_handle);
                            }
                        });
                        
                        break;
                    }
                }

                if !io_status_block.is_null() {
                    (*io_status_block).Information = current_offset;
                    (*io_status_block).Anonymous.Pointer = std::ptr::null_mut();
                    (*io_status_block).Anonymous.Status = if entries_written > 0 {
                        windows_sys::Win32::Foundation::STATUS_SUCCESS
                    } 
                    else {
                        windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES
                    };
                }

                return if entries_written > 0 {
                    windows_sys::Win32::Foundation::STATUS_SUCCESS
                } 
                else {
                    windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES
                };
            }
            else {
                //second or subsequent query no cache

                //crate::logger::output_debug_string(&format!("nt_query_directory_file can't find dir: {:#?} handle dir len: {}", file_handle, NT_HANDLE_AND_DIR.with(|cell| cell.borrow().len())));
                
                let handle_cache_dir = NT_HANDLE_AND_DIR.with(|cell| {
                    let handle_and_dir = cell.borrow();
                    return handle_and_dir.get(&file_handle).is_some();
                });

                if handle_cache_dir {
                    NT_HANDLE_AND_DIR.with(|cell| {
                        let mut handle_and_dir = cell.borrow_mut();
                        handle_and_dir.remove(&file_handle);
                    });

                    if !io_status_block.is_null() {
                        (*io_status_block).Information = 0;
                        (*io_status_block).Anonymous.Pointer = std::ptr::null_mut();
                        (*io_status_block).Anonymous.Status = windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES;
                    }
                    return windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES;
                }
                else {
                    let nt_status = nt_query_directory_file(
                        file_handle,
                        event,
                        apc_routine,
                        apc_context,
                        io_status_block,
                        file_information,
                        length,
                        file_information_class,
                        return_single_entry,
                        file_name,
                        restart_scan
                    );
                    if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS && !file_information.is_null() {
                    }
                    else {
                        crate::log!(error, "nt_query_directory_file direct call failed with status: {:#X}", nt_status);
                    }

                    return nt_status;
                }
            }
        }
        else {
            //first query.
            let mut file_path: Option<String> = None;
            NT_HANDLE_AND_DIR.with(|cell| {
                let map = cell.borrow();
                crate::log!(trace, "nt_query_directory_file known file handles: {:?} {:?}", map, file_handle);
                if let Some(path) = map.get(&file_handle) {
                    file_path = Some(path.clone());
                }
            });

            if let Some(path) = file_path {

                let (tx, rx) = tokio::sync::oneshot::channel();
                let mut args =  std::collections::HashMap::<String, String>::new();
                args.insert("filehandle".to_string(), path.clone());
                args.insert("length".to_string(), length.to_string());

                if !file_name.is_null() {
                    let buffer = (*file_name).Buffer;
                    let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
                    crate::log!(trace, "nt_query_directory_file path: {:?}", name);

                    args.insert("filename".to_string(), name);
                }
                let call = {
                    let mut cid = SYS_CALL_ID.lock().unwrap();
                    let call = crate::syscallredirect::MirrorSysCall {
                        id: *cid,
                        api: "NtQueryDirectoryFile".into(),
                        args,
                        responder: tx,
                    };
                    *cid += 1;
                    call
                };
                let id = call.id.clone();

                crate::log!(trace, "nt_query_directory_file file handle id: {} path: {} process: {} thread: {:?}", id.clone(), path, std::process::id(), std::thread::current().id());
                crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(call).unwrap();
                let result = rx.blocking_recv().unwrap();
                //let result = std::collections::HashMap::<String, String>::new();
                crate::log!(trace, "nt_query_directory_file file handle id: {} path: {} results: {:?} process: {} thread: {:?}", id, path, result, std::process::id(), std::thread::current().id());

                if let Some(fileinfo) = result.get("fileinformation") {
                    if file_information_class != windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation {
                        crate::log!(warn, "unsupported file_information_class: {:?}, falling back to original", file_information_class);
                    } 
                    else {
                    
                    }
                    let mut current_offset = 0usize;

                    let filenames: Vec<String> = fileinfo.lines().map(|item| item.to_string()).collect();
                    let files_count = filenames.len();

                    let mut entries_written = 0;
                    for (index, file) in filenames.iter().enumerate() {
                        let virtual_file_name: Vec<u16> = file.encode_utf16().collect();
                        let virtual_file_name_bytes: u32 = (virtual_file_name.len() * 2) as u32;

                        let base_size = std::mem::size_of::<windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION>() - std::mem::size_of::<u16>();
                        let entry_size = base_size + virtual_file_name_bytes as usize;

                        let aligned_entry_size = (entry_size + 7) & !7; // Align to 8 bytes

                        if current_offset + aligned_entry_size as usize > length as usize {
                            crate::log!(error, "nt_query_directory_file: buffer overflow, current_offset: {}, aligned_entry_size: {}, length: {}", current_offset, aligned_entry_size, length);
                            break;
                        }

                        let entry_ptr = file_information.add(current_offset);
                        std::ptr::write_bytes(entry_ptr, 0u8, aligned_entry_size);

                        let entry = entry_ptr as *mut windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;

                        let next_offset = if index == files_count - 1 || return_single_entry {
                            0
                        } else {
                            aligned_entry_size as u32
                        };

                        (*entry).NextEntryOffset = next_offset;                        
                        (*entry).FileIndex = index as u32;
                        (*entry).FileNameLength = virtual_file_name_bytes as u32;
                        
                        std::ptr::copy_nonoverlapping(
                            virtual_file_name.as_ptr(),
                            (*entry).FileName.as_mut_ptr(),
                            virtual_file_name.len()
                        );

                        current_offset += aligned_entry_size as usize;
                        entries_written += 1;

                        if return_single_entry {
                            NT_HANDLE_AND_FILENAMES.with(|cell| {
                                let mut handle_and_filenames = cell.borrow_mut();
                                if index + 1 < files_count {
                                    handle_and_filenames.insert(file_handle, filenames[index + 1..].to_vec());
                                }
                                else {
                                    handle_and_filenames.remove(&file_handle);
                                }
                            });
                            
                            break;
                        }
                    }

                    if !io_status_block.is_null() {
                        (*io_status_block).Information = current_offset;
                        (*io_status_block).Anonymous.Pointer = std::ptr::null_mut();
                        (*io_status_block).Anonymous.Status = if entries_written > 0 {
                            windows_sys::Win32::Foundation::STATUS_SUCCESS
                        } 
                        else {
                            windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES
                        };
                    }

                    /* 
                    //try access result
                    let mut current_offset = 0usize;
                    let mut entry_count = 0;
        
                    loop {
                        let current_entry = (file_information as *const u8).add(current_offset);
                        entry_count += 1;
                        match file_information_class {
                            windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation => {
                                let file_info = current_entry as *const windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;
                                let file_name_length_bytes = (*file_info).FileNameLength as usize;


                                if file_name_length_bytes > 0 {
                                    let file_name_slice = std::slice::from_raw_parts((*file_info).FileName.as_ptr(), file_name_length_bytes / 2);
                                    if let Ok(file_name_str) = String::from_utf16(file_name_slice) {
                                        crate::log!(trace, "test entry #{}: file: '{}'", entry_count, file_name_str);
                                    }
                                }

                                let next_entry_offset = (*file_info).NextEntryOffset;
                                if next_entry_offset == 0 {
                                    break;
                                }
                                else {
                                    current_offset += next_entry_offset as usize;
                                }
                            }
                            _ => {
                                crate::log!(trace, "test nt_query_directory_file: unsupported file_information_class: {:?}", file_information_class);
                            }
                        }

                        if current_offset >= length as usize {
                            crate::log!(warn, "Test Reached buffer end, processed {} entries", entry_count);
                            break;
                        }
                    }
                    */
                        
                    return if entries_written > 0 {
                        windows_sys::Win32::Foundation::STATUS_SUCCESS
                    } 
                    else {
                        windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES
                    };
                }
                else {
                    crate::log!(error, "nt_query_directory_file: no file information received from net redirect.");
                    return windows_sys::Win32::Foundation::STATUS_NO_SUCH_FILE;
                }
            }
            else {
                crate::log!(error, "nt_query_directory_file: don't get dir by handle. {:?}, so directly call nt_query_directory_file", file_handle);

                let mut buffer: [u16; windows_sys::Win32::Foundation::MAX_PATH as usize] = [0; windows_sys::Win32::Foundation::MAX_PATH as usize];
                let required_length = windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW(
                    file_handle,
                    buffer.as_mut_ptr(),
                    windows_sys::Win32::Foundation::MAX_PATH,
                    0
                );

                if  required_length > 0 && required_length <= windows_sys::Win32::Foundation::MAX_PATH {
                    let file_path = crate::utils::convert::lpwstr_2_string(buffer.as_ptr());
                    crate::log!(trace, "nt_query_directory_file: file_path: {:?}", file_path);
                }

                let nt_status = nt_query_directory_file(
                    file_handle,
                    event,
                    apc_routine,
                    apc_context,
                    io_status_block,
                    file_information,
                    length,
                    file_information_class,
                    return_single_entry,
                    file_name,
                    restart_scan
                );
                if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS && !file_information.is_null() {
                    crate::log!(trace, "nt_query_directory_file direct call success.");
                }
                else {
                    crate::log!(error, "nt_query_directory_file direct call failed with status: {:#X}", nt_status);
                }

                return nt_status;
            }
        }
    }
}

pub unsafe fn nt_create_file(
    file_handle:         super::ntdef::types::PHANDLE,
    access_mask:         super::ntdef::types::ACCESS_MASK,
    object_attributes:   super::ntdef::structs::POBJECT_ATTRIBUTES,
    io_status_block:      super::ntdef::structs::PIO_STATUS_BLOCK,
    allocation_size:     super::ntdef::structs::PLARGE_INTEGER,
    file_attributes:     super::ntdef::types::ULONG,
    share_access:        super::ntdef::types::ULONG,
    create_disposition:  super::ntdef::types::ULONG,
    create_options:      super::ntdef::types::ULONG,
    ea_buffer:           super::ntdef::types::PVOID,
    ea_length:           super::ntdef::types::ULONG
    ) -> super::ntdef::types::NTSTATUS {

    use std::os::windows::ffi::OsStrExt;

    let zw_create_file: super::ntdef::functions::ZwCreateFile = std::mem::transmute(NT_CREATE_FILE);
    
    if !object_attributes.is_null() {
        let object_name = (*object_attributes).ObjectName;
        if !object_name.is_null() {
            let buffer = (*object_name).Buffer;
            let length = (*object_name).Length;

            if !buffer.is_null() && length > 0 {

                /* 
                // another way to get string from utf16 slice.
                let utf16_slice = std::slice::from_raw_parts(buffer, (length / 2) as usize);
                let os_string = std::ffi::OsString::from_wide(utf16_slice);
                match os_string.into_string() {
                    Ok(_string) => {
                        //println!("hook func nt_create_file, path {:?}, object name length {}", string, length );
                    },
                    Err(_) => {
                        crate::log!(error,"can't convert osstring into string.");
                    }
                }
                */

                let mut name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
                crate::log!(trace, "nt_create_file hook path: {}", name);
                let replace = crate::replace::replace_dir(&mut name);
                if replace == crate::replace::ReplaceDirResult::Success {
                    //TODO elpase 10ms, need optimize. 
                    crate::log!(trace, "nt_create_file replace hook: {}", name.clone());

                    let mut object_name: windows_sys::Win32::Foundation::UNICODE_STRING = std::mem::zeroed();
                    let object_name_source_wide_char = std::ffi::OsString::from(name.clone()).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
                    
                    let ret = windows_sys::Wdk::Storage::FileSystem::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
                    if ret != windows_sys::Win32::Foundation::STATUS_SUCCESS {
                        crate::log!(error, "rtl init unicode string failed.");
                    }

                    let mut fake_obejct_name_adapter = crate::ntdef::structs::UNICODE_STRING {
                       Length: object_name.Length,
                       MaximumLength: object_name.MaximumLength,
                       Buffer: object_name.Buffer,
                    };

                    (*object_attributes).ObjectName = &mut fake_obejct_name_adapter;

                    //crate::log!(debug, "nt_create_file re hook: {:?}", crate::utils::convert::lpwstr_2_string((*(*object_attributes).ObjectName).Buffer).unwrap());
                    //crate::log!(debug, "nt_create_file re hook: {:?}", crate::utils::convert::lpwstr_2_string(object_name_source_wide_char.as_ptr()).unwrap());

                    let mut nt_status = zw_create_file(
                        file_handle,
                        access_mask,
                        object_attributes,
                        io_status_block,
                        allocation_size,
                        file_attributes,
                        share_access,
                        create_disposition,
                        create_options,
                        ea_buffer,
                        ea_length
                    );

                    if nt_status == winapi::shared::ntstatus::STATUS_SUCCESS {
                        crate::log!(trace, "nt_create_file file_handle: {:#?}", *file_handle);
                        NT_HANDLE_AND_DIR.with(|cell| {
                            cell.borrow_mut().insert(*file_handle as windows_sys::Win32::Foundation::HANDLE, name);
                        });
                    }
                    else {
                        if nt_status == winapi::shared::ntstatus::STATUS_OBJECT_NAME_NOT_FOUND {

                        }
                        else {
                            crate::log!(error, "zw_create_file failed! error_code: {:#X} path: {}", nt_status, name);

                            if nt_status == winapi::shared::ntstatus::STATUS_SHARING_VIOLATION {
                                nt_status = zw_create_file(
                                    file_handle,
                                    access_mask,
                                    object_attributes,
                                    io_status_block,
                                    allocation_size,
                                    file_attributes,
                                    share_access,
                                    create_disposition,
                                    create_options,
                                    ea_buffer,
                                    ea_length
                                );
                            }
                        }
                    }
                    return nt_status;
                }
                else if replace == crate::replace::ReplaceDirResult::IncludesDir {
                    let nt_status = zw_create_file(
                        file_handle,
                        access_mask,
                        object_attributes,
                        io_status_block,
                        allocation_size,
                        file_attributes,
                        share_access,
                        create_disposition,
                        create_options,
                        ea_buffer,
                        ea_length
                    );
                    
                    if nt_status == winapi::shared::ntstatus::STATUS_SUCCESS {
                        crate::log!(trace, "nt_create_file include dir: tid: {:?} {:#?} {}", std::thread::current().id(), *file_handle, name);
                        NT_HANDLE_AND_DIR.with(|cell| {
                            cell.borrow_mut().insert(*file_handle as windows_sys::Win32::Foundation::HANDLE, name);
                            crate::log!(trace, "nt_create_file include dir len: {:?}", cell.borrow().len());
                        });
                    }

                    return nt_status;
                }
                else if let crate::replace::ReplaceDirResult::NeedObtain(expect) = replace {
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    let mut args =  std::collections::HashMap::<String, String>::new();
                    args.insert("objectname".to_string(), name.clone());
                    args.insert("expect".to_string(), expect.clone());

                    let item = {
                        let guard = INCLUDES_CACHE.lock().unwrap();
                        guard.get(&name).cloned()
                    };

                    if let Some(expect) = item {
                        crate::log!(trace, "nt_create_file redirect file handle path by cache expect: {}", expect);
                        let mut object_name: windows_sys::Win32::Foundation::UNICODE_STRING = std::mem::zeroed();
                        //let expect = format!(r"\??\{}", expect);
                        let object_name_source_wide_char = std::ffi::OsString::from(&expect).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
                        
                        let ret = windows_sys::Wdk::Storage::FileSystem::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
                        if ret != windows_sys::Win32::Foundation::STATUS_SUCCESS {
                            crate::log!(error, "rtl init unicode string failed.");
                        }

                        let mut expect_obejct_name_adapter = crate::ntdef::structs::UNICODE_STRING {
                            Length: object_name.Length,
                            MaximumLength: object_name.MaximumLength,
                            Buffer: object_name.Buffer,
                        };

                        (*object_attributes).ObjectName = &mut expect_obejct_name_adapter;
                        let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                            allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                        );

                        if nt_status == winapi::shared::ntstatus::STATUS_SUCCESS {
                    
                        }
                        else {
                            if nt_status == winapi::shared::ntstatus::STATUS_SHARING_VIOLATION {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                                    allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                                );
                                if nt_status == winapi::shared::ntstatus::STATUS_SUCCESS {
                                   
                                }
                                else {
                                    let object_name = (*object_attributes).ObjectName;
                                    if !object_name.is_null() {
                                        let buffer = (*object_name).Buffer;
                                        let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
                                        crate::log!(error, "zw_create_file with expect failed! error_code: {:#X} expect: {}", nt_status, name);
                                    }
                                    else {
                                        crate::log!(error, "zw_create_file with expect failed! error_code: {:#X} expect: <null>", nt_status);
                                    } 
                                }
                                return nt_status;
                            }
                        }
                        return nt_status;
                    }
                    else {
                        let syscall = {
                            let mut id = SYS_CALL_ID.lock().unwrap();
                            let syscall = crate::syscallredirect::MirrorSysCall {
                                id: *id,
                                api: "NtCreateFile".into(),
                                args,
                                responder: tx,
                            };
                            *id += 1;
                            syscall
                        };

                        let now = std::time::Instant::now();
                        crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(syscall).unwrap();
                        let expects = rx.blocking_recv().unwrap();
                        //let expects = std::collections::HashMap::<String, String>::new();
                        crate::log!(trace, "nt_create_file redirect file handle path by sync result: {} elapsed: {:?}", name, now.elapsed());
                        if let Some(expect) = expects.get("expect") {
                            
                            INCLUDES_CACHE.lock().unwrap().insert(name, expect.clone());

                            crate::log!(trace, "nt_create_file redirect file handle path by sync expect: {}", expect);
                            let mut object_name: windows_sys::Win32::Foundation::UNICODE_STRING = std::mem::zeroed();
                            //let expect = format!(r"\??\{}", expect);
                            let object_name_source_wide_char = std::ffi::OsString::from(&expect).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
                            
                            let ret = windows_sys::Wdk::Storage::FileSystem::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
                            if ret != windows_sys::Win32::Foundation::STATUS_SUCCESS {
                                crate::log!(error, "rtl init unicode string failed.");
                            }

                            let mut expect_obejct_name_adapter = crate::ntdef::structs::UNICODE_STRING {
                                Length: object_name.Length,
                                MaximumLength: object_name.MaximumLength,
                                Buffer: object_name.Buffer,
                            };

                            (*object_attributes).ObjectName = &mut expect_obejct_name_adapter;

                            let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                                allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                            );

                            if nt_status == winapi::shared::ntstatus::STATUS_SUCCESS {
                        
                            }
                            else {
                                if nt_status == winapi::shared::ntstatus::STATUS_SHARING_VIOLATION {
                                    std::thread::sleep(std::time::Duration::from_millis(5));
                                    let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                                        allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                                    );
                                    if nt_status == winapi::shared::ntstatus::STATUS_SUCCESS {
                                        // Handle success case
                                    }
                                    else {
                                        let object_name = (*object_attributes).ObjectName;
                                        if !object_name.is_null() {
                                            let buffer = (*object_name).Buffer;
                                            let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
                                            crate::log!(error, "zw_create_file with expect failed! error_code: {:#X} expect: {} access: {}", nt_status, name, share_access);
                                        }
                                        else {
                                            crate::log!(error, "zw_create_file with expect failed! error_code: {:#X} expect: <null>", nt_status);
                                        } 
                                    }
                                    return nt_status;
                                }
                            }
                            return nt_status;
                        }
                    }
                }
                else {
                    let nt_status = zw_create_file(
                        file_handle,
                        access_mask,
                        object_attributes,
                        io_status_block,
                        allocation_size,
                        file_attributes,
                        share_access,
                        create_disposition,
                        create_options,
                        ea_buffer,
                        ea_length
                    );
                    return nt_status;
                }
            }
        }
    }

    let nt_status = zw_create_file(
        file_handle,
        access_mask,
        object_attributes,
        io_status_block,
        allocation_size,
        file_attributes,
        share_access,
        create_disposition,
        create_options,
        ea_buffer,
        ea_length
    );
    return nt_status;
}

pub unsafe fn pass_project_and_replica_to_redriect(handle: winapi::shared::ntdef::HANDLE, solution: &str, project: &str, replica: &str) {
    if !project.is_empty() {
        let arg = format!("solution:{}\nproject:{}\nreplica:{}\n", solution, project, replica);
        let mut bytes: winapi::shared::minwindef::DWORD = 0;
        let mut overlapped: winapi::um::minwinbase::OVERLAPPED = std::mem::zeroed();
        let ret = winapi::um::fileapi::WriteFile(
            handle,
            arg.as_bytes().as_ptr() as *const winapi::ctypes::c_void,
            arg.len() as u32,
            &mut bytes,
            &mut overlapped
        );
    
        if ret == winapi::shared::minwindef::FALSE || bytes == 0 {
            let error = winapi::um::errhandlingapi::GetLastError();
            crate::log!(error, "write pipe error, failed code: {}", error);
        }
        else {
            //winapi::um::fileapi::FlushFileBuffers(handle);
            crate::log!(trace, "childprocess send message by pipe {} {:?}", if bytes > 0 {"success."} else {"failed."}, arg);
        }
    }
    else {
        crate::log!(warn, "don't pass project name and project path, use current path and don't redirect.");
    }
    winapi::um::handleapi::CloseHandle(handle);
}

#[cfg(test)]
mod tests {
    use std::os::windows::ffi::OsStrExt;
    use super::*;
    #[test]
    fn start_with_pipe_test() {
     
        let mut pipe: Vec<u16> = std::ffi::OsStr::new("\\\\.\\pipe\\").encode_wide().collect();
        pipe.push(0); 
        unsafe {
            let new = std::time::Instant::now();
            let result = start_with_pipe_w(pipe.as_ptr());
            println!("start_with_pipe func elapsed time: {:?}", new.elapsed());
            assert_eq!(result, true);
        }

        let mut pipe: Vec<u16> = std::ffi::OsStr::new("\\\\.\\pipe\\pipename").encode_wide().collect();
        pipe.push(0); 
        unsafe {
            let result = start_with_pipe_w(pipe.as_ptr());
            assert_eq!(result, true);
        }


        let mut pipe: Vec<u16> = std::ffi::OsStr::new(r"\\.\test").encode_wide().collect();
        pipe.push(0);
        unsafe {
            let result = start_with_pipe_w(pipe.as_ptr() as *const u16);
            assert_eq!(result, false);
        }
        
        let mut pipe: Vec<u16> = std::ffi::OsStr::new(r"\.\pipe").encode_wide().collect();
        pipe.push(0);
        unsafe {
            let result = start_with_pipe_w(pipe.as_ptr() as *const u16);
            assert_eq!(result, false);
        }


        let pipe = std::ffi::CString::new(r"").unwrap();
        unsafe {
            let result = start_with_pipe_w(pipe.as_ptr() as *const u16);
            assert_eq!(result, false);
        }

        let pipe = std::ffi::CString::new(r"../../build/config/warning_suppression.txt").unwrap();
        unsafe {
            let result = start_with_pipe_w(pipe.as_ptr() as *const u16);
            assert_eq!(result, false);
        }
    }
}