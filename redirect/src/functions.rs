use windows_sys::Win32 as win;

use crate::log;

pub static mut CREATE_FILE_A: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_A_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_QUERY_DIRECTORY_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_CREATE_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_QUERY_INFORMATION_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_QUERY_VOLUME_INFORMATION_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_QUERY_FULL_ATTRIBUTES_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_PROCESS_A_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_PROCESS_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut GET_VOLUME_INFORMATION_BY_HANDLE_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut GET_FILE_INFORMATION_BY_HANDLE_EX_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_CLOSE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;

static SYS_CALL_ID: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<u32>>> = std::sync::LazyLock::new(|| {
    let pid = std::process::id();
    std::sync::Arc::new(std::sync::Mutex::new(pid * 10000))
});

static INCLUDES_CACHE: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>> = std::sync::LazyLock::new(|| {
    std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))
});

static CALL_TEMPLATE: std::sync::LazyLock<u64> = std::sync::LazyLock::new(|| {
    let hex = "tb".as_bytes().iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    u64::from_str_radix(&hex, 16)
        .unwrap_or(0)
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
    lp_file_name: windows_sys::core::PCSTR,
    dw_desired_access: u32,
    dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
    lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
    dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    h_template_file: win::Foundation::HANDLE,
) -> win::Foundation::HANDLE {

    let path = crate::utils::convert::lpstr_2_string(lp_file_name as *const i8);
  
    if let Ok(mut path) = path {
        crate::log!(trace, "create_file_a hook path: {}", path);

        let create_file_a: extern "system" fn(
            lp_file_name: windows_sys::core::PCSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_A);

        let replace = crate::replace::replace(&mut path);
        if replace == crate::replace::ReplaceResult::Success{
            crate::log!(trace, "create_file_a replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpstr(path);

            let handle = create_file_a(
                fake_path as windows_sys::core::PCSTR,
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

            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let error_code = win::Foundation::GetLastError();
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

            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "create_file_a failed! error code: {}.", error_code)
            }

            return handle;
        }
    }
    else {
        return 0 as win::Foundation::HANDLE;
    }
}

pub unsafe fn create_file_w(
    lp_file_name: windows_sys::core::PCWSTR,
    dw_desired_access: u32,
    dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
    lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
    dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    h_template_file: win::Foundation::HANDLE,
) -> win::Foundation::HANDLE {
    if h_template_file == *CALL_TEMPLATE as win::Foundation::HANDLE {
        let path = crate::utils::convert::lpwstr_2_string(lp_file_name);
        crate::log!(trace, "create_file_w pipe path: {:?}", path);
        let create_file_w_inner: extern "system" fn (
            lp_file_name: windows_sys::core::PCWSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_W);

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
    //if start_with_pipe_w(lp_file_name) {
    else if dw_desired_access & 0x40000000 != 0 && dw_desired_access & 0x80000000 == 0 && dw_share_mode == 0 {
        let path = crate::utils::convert::lpwstr_2_string(lp_file_name);
        crate::log!(trace, "create_file_w pipe path: {:?}", path);
        let create_file_w_inner: extern "system" fn (
            lp_file_name: windows_sys::core::PCWSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_W);

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
    else if dw_desired_access & 0x40000000 == 0 && dw_desired_access & 0x80000000 != 0
        && dw_share_mode & windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ != 0 {

        let path = crate::utils::convert::lpwstr_2_string(lp_file_name);
        crate::log!(trace, "create_file_w temp path: {:?}", path);
        let create_file_w_inner: extern "system" fn (
            lp_file_name: windows_sys::core::PCWSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_W);

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
            lp_file_name: windows_sys::core::PCWSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_W);

        let replace = crate::replace::replace(&mut path);

        if replace == crate::replace::ReplaceResult::Success {
            crate::log!(trace, "create_file_w replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpwstr(path);
                
            let handle = create_file_w(
                fake_path.as_ptr(),
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );

            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = win::Foundation::GetLastError();
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

            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }

            return handle;    
        }
    }
    else {
        return 0 as win::Foundation::HANDLE;
    }

}

pub unsafe fn kernelbase_create_file_a(
    lp_file_name: windows_sys::core::PCSTR,
    dw_desired_access: u32,
    dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
    lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
    dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    h_template_file: win::Foundation::HANDLE,
) -> win::Foundation::HANDLE {

    let path = crate::utils::convert::lpstr_2_string(lp_file_name as *const i8);
  
    if let Ok(mut path) = path {
        
        crate::log!(trace, "kernelbase_create_file_a hook path: {}", path);

        let create_file_a: extern "system" fn(
            lp_file_name: windows_sys::core::PCSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_A_KERNEL_BASE);

        let replace = crate::replace::replace(&mut path);

        if replace == crate::replace::ReplaceResult::Success {
            crate::log!(trace, "kernelbase_create_file_a replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpstr(path);
            let handle = create_file_a(
                fake_path as windows_sys::core::PCSTR,
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

            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let error_code = win::Foundation::GetLastError();
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

            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "kernelbase create_file_a failed! error_code: {}.", error_code);
            }
            return handle;    
        }
    }
    else {
        return 0 as win::Foundation::HANDLE;
    }
}

pub unsafe fn kernelbase_create_file_w(
    lp_file_name: windows_sys::core::PCWSTR,
    dw_desired_access: u32,
    dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
    lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
    dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    h_template_file: win::Foundation::HANDLE,
) -> win::Foundation::HANDLE {

    if start_with_pipe_w(lp_file_name) {
        let create_file_w_inner: extern "system" fn (
            lp_file_name: windows_sys::core::PCWSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_W_KERNEL_BASE);

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
            return win::Foundation::INVALID_HANDLE_VALUE;
        }

        let create_file_w: extern "system" fn (
            lp_file_name: windows_sys::core::PCWSTR,
            dw_desired_access: u32,
            dw_share_mode: win::Storage::FileSystem::FILE_SHARE_MODE,
            lp_security_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
            dw_creation_disposition: win::Storage::FileSystem::FILE_CREATION_DISPOSITION,
            dw_flags_and_attributes: win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            h_template_file: win::Foundation::HANDLE,
        ) -> win::Foundation::HANDLE = std::mem::transmute(CREATE_FILE_W_KERNEL_BASE);

        //crate::log!(trace, "kernelbase_create_file_w hook path: {}", path);
        let replace = crate::replace::replace(&mut path);
        if replace == crate::replace::ReplaceResult::Success {
            crate::log!(trace, "kernelbase_create_file_w replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpwstr(path);
            
            let handle = create_file_w(
                fake_path.as_ptr(),
                dw_desired_access,
                dw_share_mode,
                lp_security_attributes,
                dw_creation_disposition,
                dw_flags_and_attributes,
                h_template_file,
            );
    
            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "kernelbase create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }
    
            return handle; 
        }
        else if replace == crate::replace::ReplaceResult::FilePath {
            crate::log!(trace, "kernelbase_create_file_w replace hook: {}", path);
            let fake_path = crate::utils::convert::string_2_lpwstr(path.clone());

            let handle = create_file_w(fake_path.as_ptr() as windows_sys::core::PCWSTR,
                dw_desired_access, dw_share_mode, lp_security_attributes,
                dw_creation_disposition, dw_flags_and_attributes, h_template_file,
            );
    
            if handle == win::Foundation::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = win::Foundation::GetLastError();

                if error_code == win::Foundation::ERROR_FILE_NOT_FOUND {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let mut args =  std::collections::HashMap::<String, String>::new();
                    args.insert("filename".to_string(), option_path.unwrap());
                    args.insert("expect".to_string(), path.clone());

                    let call = {
                        let mut cid = SYS_CALL_ID.lock().unwrap();
                        let call = crate::syscallredirect::MirrorSysCall {
                            cid: *cid,
                            api: "CreateFileW".into(),
                            args,
                            responder: tx,
                        };
                        *cid += 1;
                        call
                    };
                    let cid = call.cid.clone();
                    crate::log!(trace, "kernelbase_create_file_w file cid: {} path: {} process: {}", cid.clone(), path, std::process::id());

                    crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(call).unwrap();
                    let expects = rx.blocking_recv().unwrap();
                    if let Some(_) = expects.get("expect") {
                        let handle = create_file_w(fake_path.as_ptr(), dw_desired_access, dw_share_mode, lp_security_attributes,
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
    
            if handle ==  win::Foundation::INVALID_HANDLE_VALUE {
                let hook_path = crate::utils::convert::lpwstr_2_string(lp_file_name);
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "kernelbase create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
            }
            return handle;   
        }
    }
    else {
        return 0 as win::Foundation::HANDLE;
    }

}

pub unsafe fn kernelbase_create_process_a(
    lp_application_name: windows_sys::core::PCSTR,
    lp_command_line: windows_sys::core::PSTR,
    lp_process_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    lp_thread_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    b_inherit_handles: windows_sys::core::BOOL,
    dw_creation_flags: win::System::Threading::PROCESS_CREATION_FLAGS,
    lp_environment: *const core::ffi::c_void,
    lp_current_directory: windows_sys::core::PCSTR,
    lp_startup_info: *mut win::System::Threading::STARTUPINFOA,
    lp_process_information: *mut win::System::Threading::PROCESS_INFORMATION,
) -> windows_sys::core::BOOL {
    let application = crate::utils::convert::lpstr_2_string(lp_application_name as *const i8);
  
    if let Ok(application) = application {
        
        crate::log!(info, "kernelbase_create_process_a hook path: {}", application);
        //crate::log!(info, "kernelbase_create_process_a hook commandline: {:?}", crate::utils::convert::lpstr_2_string(lp_command_line));

        let dllpath = crate::MODULE_PATH.get();

        if application.ends_with("cl.exe") && dllpath.is_some() && crate::IN_HOOK.get() == false {
            crate::IN_HOOK.set(true);

            let mut stdin_write_handle: Option<win::Foundation::HANDLE> = None;

            if ((*lp_startup_info).dwFlags & win::System::Threading::STARTF_USESTDHANDLES) != 0 {

                let mut pipe_attributes = win::Security::SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<win::Security::SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: std::ptr::null_mut(),
                    bInheritHandle: win::Foundation::TRUE,
                };

                let mut h_stdin_read: win::Foundation::HANDLE = std::ptr::null_mut();
                let mut h_stdin_write: win::Foundation::HANDLE = std::ptr::null_mut();

                let ret = win::System::Pipes::CreatePipe(
                    &mut h_stdin_read, 
                    &mut h_stdin_write, 
                    &mut pipe_attributes, 
                    0
                );

                if win::Foundation::FALSE == ret {
                    crate::log!(error, "create input pipe failed.")
                }
                else {
                    (*lp_startup_info).hStdInput = h_stdin_read;
                    stdin_write_handle = Some(h_stdin_write);
                }
            }

            let ret = crate::detours::DetourCreateProcessWithDllExA(
                lp_application_name as *const i8,
                lp_command_line as *mut i8,
                lp_process_attributes as *mut crate::detours::_SECURITY_ATTRIBUTES,
                lp_thread_attributes as *mut crate::detours::_SECURITY_ATTRIBUTES, 
                b_inherit_handles as i32, 
                dw_creation_flags,
                lp_environment as *mut std::ffi::c_void,
                lp_current_directory as *const i8,
                lp_startup_info as crate::detours::LPSTARTUPINFOA,
                lp_process_information as crate::detours::LPPROCESS_INFORMATION, 
                dllpath.unwrap().as_ptr() as *const i8,
                Option::None
            );
            
            if ret == win::Foundation::TRUE {
                if let Some(stdin_write) = stdin_write_handle {
                    pass_project_and_replica_to_redriect(stdin_write, crate::SOLUTIONNAME.get().unwrap(), crate::PROJECTNAME.get().unwrap(), crate::REPLICADIR.get().unwrap());
                }

                return ret;
            }
            else {
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "detour create process withdllexa failed! error_code: {}.", error_code);

                let create_process_a: extern "system" fn(
                    lp_application_name: windows_sys::core::PCSTR,
                    lp_command_line: windows_sys::core::PSTR,
                    lp_process_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
                    lp_thread_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
                    b_inherit_handles: windows_sys::core::BOOL,
                    dw_creation_flags: win::System::Threading::PROCESS_CREATION_FLAGS,
                    lp_environment:  *const core::ffi::c_void,
                    lp_current_directory: windows_sys::core::PCSTR,
                    lp_startup_info: *const win::System::Threading::STARTUPINFOA,
                    lp_process_information: *mut win::System::Threading::PROCESS_INFORMATION,
                ) -> windows_sys::core::BOOL = std::mem::transmute(CREATE_PROCESS_A_KERNEL_BASE);

                let ret = create_process_a (
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
                
                if ret == win::Foundation::FALSE {
                    let error_code = win::Foundation::GetLastError();
                    crate::log!(error, "kernelbase_create_process_a failed! error_code: {}.", error_code);
                }
                return ret;
            }
        }
        else
        {
            let create_process_a: extern "system" fn(
                lp_application_name: windows_sys::core::PCSTR,
                lp_command_line: windows_sys::core::PSTR,
                lp_process_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
                lp_thread_attributes: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
                b_inherit_handles: windows_sys::core::BOOL,
                dw_creation_flags: win::System::Threading::PROCESS_CREATION_FLAGS,
                lp_environment:  *const core::ffi::c_void,
                lp_current_directory: windows_sys::core::PCSTR,
                lp_startup_info: *const win::System::Threading::STARTUPINFOA,
                lp_process_information: *mut win::System::Threading::PROCESS_INFORMATION,
            ) -> windows_sys::core::BOOL = std::mem::transmute(CREATE_PROCESS_A_KERNEL_BASE);
    
            let ret = create_process_a (
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
            
            if ret == win::Foundation::FALSE {
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "kernelbase_create_process_a failed! error_code: {}.", error_code);
            }
    
            return ret;
        }
    }
    else {
        return win::Foundation::FALSE;
    }
}

pub unsafe fn kernelbase_create_process_w(
    lp_application_name: windows_sys::core::PCWSTR,
    lp_command_line: windows_sys::core::PWSTR,
    lp_process_attributes: *const win::Security::SECURITY_ATTRIBUTES,
    lp_thread_attributes: *const win::Security::SECURITY_ATTRIBUTES,
    b_inherit_handles: windows_sys::core::BOOL,
    dw_creation_flags: win::System::Threading::PROCESS_CREATION_FLAGS,
    lp_environment: *const core::ffi::c_void,
    lp_current_directory: windows_sys::core::PCWSTR,
    lp_startup_info: *mut win::System::Threading::STARTUPINFOW,
    lp_process_information: *mut win::System::Threading::PROCESS_INFORMATION,
) -> windows_sys::core::BOOL {
    
    let application = crate::utils::convert::lpwstr_2_string(lp_application_name);

    if let Some(application) = application {
        
        crate::log!(info, "kernelbase_create_process_w hook path: {}", application);
        //crate::log!(info, "kernelbase_create_process_w hook commandline: {:?}", crate::utils::convert::lpwstr_2_string(lp_command_line));

        let dllpath = crate::MODULE_PATH.get();

        if (application.ends_with("cl.exe") || application.ends_with("mspdbsrv.exe")) && dllpath.is_some() && crate::IN_HOOK.get() == false {
            
            crate::IN_HOOK.set(true);
            
            let mut stdin_write_handle: Option<win::Foundation::HANDLE> = None;

            let mut pipe_attributes = win::Security::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<win::Security::SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: std::ptr::null_mut(),
                bInheritHandle: win::Foundation::TRUE,
            };

            let mut h_stdin_read: win::Foundation::HANDLE = std::ptr::null_mut();
            let mut h_stdin_write: win::Foundation::HANDLE = std::ptr::null_mut();

            let ret = win::System::Pipes::CreatePipe(
                &mut h_stdin_read, 
                &mut h_stdin_write, 
                &mut pipe_attributes, 
                0
            );

            let mut b_inherit_handles = b_inherit_handles;
            if win::Foundation::FALSE == ret {
                crate::log!(error, "create input pipe failed.")
            }
            else {

                if application.ends_with("mspdbsrv.exe") {

                    let current_stdout = win::System::Console::GetStdHandle(win::System::Console::STD_OUTPUT_HANDLE);
                    let current_stderr = win::System::Console::GetStdHandle(win::System::Console::STD_ERROR_HANDLE);
                    
                    if !current_stdout.is_null() {
                        win::Foundation::SetHandleInformation(
                            current_stdout,
                            win::Foundation::HANDLE_FLAG_INHERIT,
                            0
                        );
                    }
                    
                    if !current_stderr.is_null() {
                        win::Foundation::SetHandleInformation(
                            current_stderr,
                            win::Foundation::HANDLE_FLAG_INHERIT,
                            0
                        );
                    }

                    (*lp_startup_info).hStdOutput = std::ptr::null_mut();
                    (*lp_startup_info).hStdError = std::ptr::null_mut();
                }
                b_inherit_handles = win::Foundation::TRUE;

                win::Foundation::SetHandleInformation(h_stdin_write, win::Foundation::HANDLE_FLAG_INHERIT, 0);
                (*lp_startup_info).hStdInput = h_stdin_read;
                (*lp_startup_info).dwFlags |= win::System::Threading::STARTF_USESTDHANDLES;

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
                lp_startup_info as crate::detours::LPSTARTUPINFOW,
                lp_process_information as crate::detours::LPPROCESS_INFORMATION,
                dllpath.unwrap().as_ptr() as *const i8,
                Option::None
            );
            
            if ret == win::Foundation::TRUE {
                
                win::Foundation::CloseHandle(h_stdin_read);

                if let Some(stdin_write) = stdin_write_handle {
                    pass_project_and_replica_to_redriect(stdin_write, crate::SOLUTIONNAME.get().unwrap(), crate::PROJECTNAME.get().unwrap(), crate::REPLICADIR.get().unwrap());
                }

                return ret;
            }
            else {
                win::Foundation::CloseHandle(h_stdin_read);
                win::Foundation::CloseHandle(h_stdin_write);
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "detour create process withdllexw failed! error_code: {}.", error_code);

                let create_process_w: extern "system" fn(
                    lp_application_name: windows_sys::core::PCWSTR,
                    lp_command_line: windows_sys::core::PWSTR,
                    lp_process_attributes: *const win::Security::SECURITY_ATTRIBUTES,
                    lp_thread_attributes: *const win::Security::SECURITY_ATTRIBUTES,
                    b_inherit_handles: windows_sys::core::BOOL,
                    dw_creation_flags: win::System::Threading::PROCESS_CREATION_FLAGS,
                    lp_environment: *const core::ffi::c_void,
                    lp_current_directory: windows_sys::core::PCWSTR,
                    lp_startup_info: *const win::System::Threading::STARTUPINFOW,
                    lp_process_information: *mut win::System::Threading::PROCESS_INFORMATION,
                ) -> windows_sys::core::BOOL = std::mem::transmute(CREATE_PROCESS_W_KERNEL_BASE);

                let ret = create_process_w (
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
                
                if ret == win::Foundation::FALSE {
                    let error_code = win::Foundation::GetLastError();
                    crate::log!(error, "kernelbase create_file_w failed! error_code: {}.", error_code);
                }
                return ret;
            }
        } 
        else {
            let create_process_w: extern "system" fn(
                lp_application_name: windows_sys::core::PCWSTR,
                lp_command_line: windows_sys::core::PWSTR,
                lp_process_attributes: *const win::Security::SECURITY_ATTRIBUTES,
                lp_thread_attributes: *const win::Security::SECURITY_ATTRIBUTES,
                b_inherit_handles: windows_sys::core::BOOL,
                dw_creation_flags: win::System::Threading::PROCESS_CREATION_FLAGS,
                lp_environment: *const core::ffi::c_void,
                lp_current_directory: windows_sys::core::PCWSTR,
                lp_startup_info: *const win::System::Threading::STARTUPINFOW,
                lp_process_information: *mut win::System::Threading::PROCESS_INFORMATION,
            ) -> windows_sys::core::BOOL = std::mem::transmute(CREATE_PROCESS_W_KERNEL_BASE);
            
            let ret = create_process_w (
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
            
            if ret == win::Foundation::FALSE {
                let error_code = win::Foundation::GetLastError();
                crate::log!(error, "kernelbase create_file_w failed! error_code: {}.", error_code);
            }
            return ret;
        }
    }
    else {
        return win::Foundation::FALSE;
    }
}

pub unsafe fn kernelbase_get_volume_information_by_handle_w(
    hfile: windows_sys::Win32::Foundation::HANDLE,
    lpvolumenamebuffer: windows_sys::core::PWSTR,
    nvolumenamesize: u32,
    lpvolumeserialnumber: *mut u32,
    lpmaximumcomponentlength: *mut u32,
    lpfilesystemflags: *mut u32,
    lpfilesystemnamebuffer: windows_sys::core::PWSTR,
    nfilesystemnamesize: u32,
) -> windows_sys::core::BOOL {

    log!(trace, "kernelbase_get_volume_information_by_handle_w called: handle: {:?}", &hfile);

    let skip = NT_HANDLE_AND_DIR.with(|cell| {
        let handle_and_dir = cell.borrow();
        return handle_and_dir.get(&hfile).is_some();
    });

    if skip {
        log!(info, "kernelbase_get_volume_information_by_handle_w skip handle: {:?}", &hfile);
        return windows_sys::core::BOOL::from(true);
    }
    else {
        let get_volume_information_by_handle_w: extern "system" fn(
            hfile: windows_sys::Win32::Foundation::HANDLE,
            lpvolumenamebuffer: windows_sys::core::PWSTR,
            nvolumenamesize: u32,
            lpvolumeserialnumber: *mut u32,
            lpmaximumcomponentlength: *mut u32,
            lpfilesystemflags: *mut u32,
            lpfilesystemnamebuffer: windows_sys::core::PWSTR,
            nfilesystemnamesize: u32,
        ) -> windows_sys::core::BOOL = std::mem::transmute(GET_VOLUME_INFORMATION_BY_HANDLE_W_KERNEL_BASE);

        return get_volume_information_by_handle_w(
            hfile,
            lpvolumenamebuffer,
            nvolumenamesize,
            lpvolumeserialnumber,
            lpmaximumcomponentlength,
            lpfilesystemflags,
            lpfilesystemnamebuffer,
            nfilesystemnamesize,
        );
    }

}

pub unsafe fn kernelbase_get_file_information_by_handle_ex(
    hfile: windows_sys::Win32::Foundation::HANDLE,
    fileinformationclass: windows_sys::Win32::Storage::FileSystem::FILE_INFO_BY_HANDLE_CLASS,
    lpfileinformation: *mut core::ffi::c_void,
    dwbuffersize: u32,
) -> windows_sys::core::BOOL {

    log!(trace, "kernelbase_get_file_information_by_handle_ex called: handle: {:?}, fileinformationclass: {:?}", hfile, fileinformationclass);

    let skip = NT_HANDLE_AND_DIR.with(|cell| {
        let handle_and_dir = cell.borrow();
        return handle_and_dir.get(&hfile).is_some();
    });

    if skip {
        log!(info, "kernelbase_get_file_information_by_handle_ex skip handle: {:?}", hfile);
        return windows_sys::core::BOOL::from(true);
    }
    else  {
        let get_file_information_by_handle_ex: extern "system" fn(
            hfile: windows_sys::Win32::Foundation::HANDLE,
            fileinformationclass: windows_sys::Win32::Storage::FileSystem::FILE_INFO_BY_HANDLE_CLASS,
            lpfileinformation: *mut core::ffi::c_void,
            dwbuffersize: u32,
        ) -> windows_sys::core::BOOL = std::mem::transmute(GET_FILE_INFORMATION_BY_HANDLE_EX_KERNEL_BASE);

        return get_file_information_by_handle_ex(
            hfile,
            fileinformationclass,
            lpfileinformation,
            dwbuffersize,
        );
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

static NT_HANDLE_MAP_FILES: std::sync::LazyLock<std::sync::RwLock<std::collections::HashMap<i32, Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(std::collections::HashMap::new()));


unsafe fn dump_files(file_information: *mut core::ffi::c_void, file_information_class: windows_sys::Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS, length: u32) {
               
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
        apcroutine: windows_sys::Win32::System::IO::PIO_APC_ROUTINE,
        apccontext: *const core::ffi::c_void,
        iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
        fileinformation: *mut core::ffi::c_void,
        length: u32,
        fileinformationclass: windows_sys::Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS,
        returnsingleentry: bool,
        filename: *const windows_sys::Win32::Foundation::UNICODE_STRING,
        restartscan: bool,
    ) -> windows_sys::Win32::Foundation::NTSTATUS = std::mem::transmute(NT_QUERY_DIRECTORY_FILE);

    crate::log!(debug, "nt_query_directory_file called: handle: {:?}, file_information_class: {:?}, return_single_entry: {}, restart_scan: {}, file_name: {:?}, buffer length: {}", file_handle, file_information_class, return_single_entry, restart_scan, 
        if !file_name.is_null() {
            let buffer = (*file_name).Buffer;
            let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
            name
        }
        else {
            "nt_query_directory_file file_name buffer is null".to_string()
        }, length
    );

    if (restart_scan && !file_name.is_null()) || (file_information_class != windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation) {
      
        if let Some(val) = NT_HANDLE_MAP_FILES.read().unwrap().get(&(file_handle as i32)) {
            let buffer = (*file_name).Buffer;
            let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();

            if val.contains(&name.to_lowercase()) {
                crate::log!(debug, "nt_query_directory_file: hit cached dir file: {}", name);

                if !file_information.is_null() {
                    let virtual_file_name: Vec<u16> = name.encode_utf16().collect();
                    let virtual_file_name_bytes: u32 = (virtual_file_name.len() * 2) as u32;
                    
                    let base_size = 64 as usize;
                    let entry_size = base_size + virtual_file_name_bytes as usize;
                    let aligned_entry_size = (entry_size + 7) & !7; // Align to 8 bytes
                    
                    std::ptr::write_bytes(file_information, 0u8, aligned_entry_size);
                    let entry = file_information as *mut windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;
                    
                    (*entry).NextEntryOffset = 0;
                    (*entry).FileIndex = 0;
                    (*entry).FileNameLength = virtual_file_name_bytes as u32;

                    (*entry).FileAttributes = win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE;
                    (*entry).EndOfFile = 1024;
                    (*entry).AllocationSize = 1024;
                    let time: i64 = 0x1DB777E6D1C0000i64;
                    (*entry).CreationTime = time;
                    (*entry).LastAccessTime = time;
                    (*entry).LastWriteTime = time;
                    (*entry).ChangeTime = time;
                    
                    std::ptr::copy_nonoverlapping(
                        virtual_file_name.as_ptr(),
                        (*entry).FileName.as_mut_ptr(),
                        virtual_file_name.len()
                    );
                
                    if !io_status_block.is_null() {
                        (*io_status_block).Information = aligned_entry_size;
                        (*io_status_block).Anonymous.Pointer = std::ptr::null_mut();
                        (*io_status_block).Anonymous.Status = windows_sys::Win32::Foundation::STATUS_SUCCESS;
                    }    
                    return windows_sys::Win32::Foundation::STATUS_SUCCESS;
                }
                else {
                    crate::log!(trace, "nt_query_directory_file: buffer too small {}", name);
                    if !io_status_block.is_null() {
                        (*io_status_block).Information = 0;
                        (*io_status_block).Anonymous.Pointer = std::ptr::null_mut();
                        (*io_status_block).Anonymous.Status = windows_sys::Win32::Foundation::STATUS_BUFFER_TOO_SMALL;
                    }
                    return windows_sys::Win32::Foundation::STATUS_BUFFER_TOO_SMALL;
                }
            }
            else {
                let nt_status = nt_query_directory_file(file_handle, event, apc_routine, apc_context, io_status_block,
                    file_information, length, file_information_class, return_single_entry, file_name, restart_scan
                );
                return nt_status;
            }
        }
        else {
            let buffer = (*file_name).Buffer;
            let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();

            let nt_status = nt_query_directory_file(file_handle, event, apc_routine, apc_context, io_status_block,
                file_information, length, file_information_class, return_single_entry, file_name, restart_scan
            );

            if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS && !file_information.is_null() {
                //dump_files(file_information, file_information_class, length);
            }
            else {
                if nt_status == windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES {
                    crate::log!(debug, "nt_query_directory_file no more files. {} handle: {:?}", name, file_handle);
                }
                else if nt_status == windows_sys::Win32::Foundation::STATUS_NO_SUCH_FILE {
                    crate::log!(warn, "nt_query_directory_file no such file. {} handle: {:?}", name, file_handle);
                }
                else if nt_status == windows_sys::Win32::Foundation::STATUS_BUFFER_OVERFLOW {
                    crate::log!(warn, "nt_query_directory_file buffer overflow occurred, consider increasing buffer size. {} handle: {:?}", name, file_handle);
                }
                else {
                    crate::log!(error, "nt_query_directory_file failed with status: {:#X} file: {:?}", nt_status, name);
                }
            }

            return nt_status;
        }
    }
    else {
        //second or subsequent query.
        if (*io_status_block).Information < length as usize && (*io_status_block).Information > 0 && (*io_status_block).Anonymous.Status == windows_sys::Win32::Foundation::STATUS_SUCCESS {

            crate::log!(debug, "nt_query_directory_file second or subsequent query handle: {:?}, Information: {} length: {} status: {:#X}", file_handle, (*io_status_block).Information, length, (*io_status_block).Anonymous.Status);

            let maybe_filenames = NT_HANDLE_AND_FILENAMES.with(|cell| {
                let handle_and_filenames = cell.borrow();
                handle_and_filenames.get(&file_handle).cloned()
            });

            //second or subsequent query with filenames cache
            if let Some(filenames) = maybe_filenames {
                let mut entries_written = 0;
                let mut current_offset = 0usize;
                let mut islastone = false;
                let last_file_attribute_archive = win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE + 1;
                let mut collectfiles: Vec<String> = Vec::new();
                for (index, fileinfo) in filenames.iter().enumerate() {

                    let fileinfo = fileinfo.split('|').collect::<Vec<&str>>();
     
                    let virtual_file_name: Vec<u16> = fileinfo[0].encode_utf16().collect();
                    let virtual_file_name_bytes: u32 = (virtual_file_name.len() * 2) as u32;

                    //let base_size = std::mem::size_of::<windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION>() - std::mem::size_of::<u16>();
                    let base_size = 64 as usize;
                    let entry_size = base_size + virtual_file_name_bytes as usize;

                    let aligned_entry_size = (entry_size + 7) & !7; // Align to 8 bytes

                    if current_offset + aligned_entry_size as usize > length as usize {
                        crate::log!(error, "nt_query_directory_file: buffer overflow, current_offset: {}, aligned_entry_size: {}, length: {}", current_offset, aligned_entry_size, length);
                        break;
                    }

                    let entry_ptr = file_information.add(current_offset);
                    std::ptr::write_bytes(entry_ptr, 0u8, aligned_entry_size);
                    
                    let entry = entry_ptr as *mut windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;
                    
                    let mut attr = fileinfo[1].parse::<u32>().unwrap_or(win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE);
                    if attr == win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE {
                        collectfiles.push(fileinfo[0].to_lowercase());
                    }
                    else if attr == last_file_attribute_archive {
                        islastone = true;
                        attr = win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE;
                        collectfiles.push(fileinfo[0].to_lowercase());
                    }

                    entries_written = index + 1;

                    let next_offset = if entries_written == filenames.len() || islastone || return_single_entry {
                        0
                    } 
                    else {
                        aligned_entry_size as u32
                    };

                    (*entry).NextEntryOffset = next_offset;                 
                    (*entry).FileIndex = index as u32;
                    (*entry).FileNameLength = virtual_file_name_bytes as u32;

                    let end_of_file: i64 = if attr == win::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY { 0 } else { 1024 };
                    let alloc_size: i64 = if attr == win::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY { 0 } else { 1024 };
                    let time: i64 = 0x1DB777E6D1C0000i64;

                    (*entry).FileAttributes = attr;
                    (*entry).EndOfFile = end_of_file;
                    (*entry).AllocationSize = alloc_size;
                    (*entry).CreationTime = time;
                    (*entry).LastAccessTime = time;
                    (*entry).LastWriteTime = time;
                    (*entry).ChangeTime = time;
                    
                    std::ptr::copy_nonoverlapping(
                        virtual_file_name.as_ptr(),
                        (*entry).FileName.as_mut_ptr(),
                        virtual_file_name.len()
                    );

                    current_offset += aligned_entry_size as usize;

                    if return_single_entry || islastone {
                        break;
                    }
                }

                NT_HANDLE_AND_FILENAMES.with(|cell| {
                    let mut handle_and_filenames = cell.borrow_mut();
                    if entries_written < filenames.len() {
                        handle_and_filenames.insert(file_handle, filenames[entries_written..].to_vec());
                    }
                    else {
                        handle_and_filenames.remove(&file_handle);
                    }
                });

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

                    if let Some(val) = NT_HANDLE_MAP_FILES.read().unwrap().get(&(file_handle as i32)) {
                        collectfiles.extend(val.iter().cloned());
                    }

                    NT_HANDLE_MAP_FILES.write().unwrap().insert(file_handle as i32, collectfiles);

                    windows_sys::Win32::Foundation::STATUS_SUCCESS
                } 
                else {
                    NT_HANDLE_MAP_FILES.write().unwrap().remove(&(file_handle as i32));
                    windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES
                };
            }
            else {
                //second or subsequent query no filenames cache

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
                        if nt_status == windows_sys::Win32::Foundation::STATUS_NO_MORE_FILES {
                        }
                        else if nt_status == windows_sys::Win32::Foundation::STATUS_BUFFER_OVERFLOW {
                            crate::log!(warn, "nt_query_directory_file buffer overflow occurred, consider increasing buffer size.");
                        }
                        else {
                            let name = if !file_name.is_null() {
                                let buffer = (*file_name).Buffer;
                                let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
                                name
                            }
                            else {
                                "nt_query_directory_file file_name buffer is null".to_string()
                            };

                            crate::log!(error, "nt_query_directory_file direct call failed with status: {:#X} file: {:?}", nt_status, name);
                        }
                    }
                    return nt_status;
                }
            }
        }
        else {
            //first query.

            crate::log!(trace, "nt_query_directory_file first query handle: {:?}", file_handle);

            let mut file_path: Option<String> = None;
            NT_HANDLE_AND_DIR.with(|cell| {
                let map = cell.borrow();
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
                        cid: *cid,
                        api: "NtQueryDirectoryFile".into(),
                        args,
                        responder: tx,
                    };
                    *cid += 1;
                    call
                };
                let cid = call.cid.clone();

                crate::log!(trace, "nt_query_directory_file file handle cid: {} path: {}", cid.clone(), path);
                crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(call).unwrap();
                let result = rx.blocking_recv().unwrap();
                crate::log!(trace, "nt_query_directory_file file handle cid: {} path: {} results: {:?}", cid, path, result);

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

                    let mut collectfiles = Vec::new();
                    let mut islastone = false;
                    let last_file_attribute_archive = win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE + 1;
                    for (index, fileinfo) in filenames.iter().enumerate() {
                        let fileinfo = fileinfo.split('|').collect::<Vec<&str>>();
     
                        let virtual_file_name: Vec<u16> = fileinfo[0].encode_utf16().collect();
                        let virtual_file_name_bytes: u32 = (virtual_file_name.len() * 2) as u32;

                        //let base_size = std::mem::size_of::<windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION>() - std::mem::size_of::<u16>();

                        let base_size = 64 as usize;
                        let entry_size = base_size + virtual_file_name_bytes as usize;

                        let aligned_entry_size = (entry_size + 7) & !7; // Align to 8 bytes

                        let entry_ptr = file_information.add(current_offset);
                        std::ptr::write_bytes(entry_ptr, 0u8, aligned_entry_size);

                        let entry = entry_ptr as *mut windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;

                        let mut attr = fileinfo[1].parse::<u32>().unwrap_or(win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE);
                        if attr == win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE {
                            collectfiles.push(fileinfo[0].to_lowercase());
                        } 
                        else if attr == last_file_attribute_archive {
                            islastone = true;
                            attr = win::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE;
                            collectfiles.push(fileinfo[0].to_lowercase());
                        }
                        entries_written = index + 1;
                        let next_offset = if entries_written == files_count || islastone || return_single_entry {
                            0
                        } 
                        else {
                            aligned_entry_size as u32
                        };

                        (*entry).NextEntryOffset = next_offset;                      
                        (*entry).FileIndex = index as u32;
                        (*entry).FileNameLength = virtual_file_name_bytes as u32;
                        
                        let end_of_file: i64 = if attr == win::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY { 0 } else { 1024 };
                        let alloc_size: i64 = if attr == win::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY { 0 } else { 1024 };
                        let time: i64 = 0x1DB777E6D1C0000i64;

                        (*entry).FileAttributes = attr;
                        (*entry).EndOfFile = end_of_file;
                        (*entry).AllocationSize = alloc_size;
                        (*entry).CreationTime = time;
                        (*entry).LastAccessTime = time;
                        (*entry).LastWriteTime = time;
                        (*entry).ChangeTime = time;
                        
                        std::ptr::copy_nonoverlapping(
                            virtual_file_name.as_ptr(),
                            (*entry).FileName.as_mut_ptr(),
                            virtual_file_name.len()
                        );

                        current_offset += aligned_entry_size as usize;

                        if return_single_entry || islastone {                      
                            break;
                        }
                    }

                    NT_HANDLE_AND_FILENAMES.with(|cell| {
                        let mut handle_and_filenames = cell.borrow_mut();
                        if entries_written < files_count {
                            handle_and_filenames.insert(file_handle, filenames[entries_written..].to_vec());
                        }
                        else {
                            handle_and_filenames.remove(&file_handle);
                        }
                    });

                    NT_HANDLE_MAP_FILES.write().unwrap().insert(file_handle as i32, collectfiles);

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
                                        crate::log!(trace, "Entry #{}: FileName='{}', FileIndex={}, NextOffset={}, FileAttrs={:#X}, EndOfFile={}, AllocSize={}, CreationTime={:#X}, LastAccessTime={:#X}, LastWriteTime={:#X}, ChangeTime={:#X}", 
                                            entry_count, 
                                            file_name_str,
                                            (*file_info).FileIndex,
                                            (*file_info).NextEntryOffset,
                                            (*file_info).FileAttributes,
                                            (*file_info).EndOfFile,
                                            (*file_info).AllocationSize,
                                            (*file_info).CreationTime,
                                            (*file_info).LastAccessTime,
                                            (*file_info).LastWriteTime,
                                            (*file_info).ChangeTime
                                        );
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
                let mut buffer: [u16; windows_sys::Win32::Foundation::MAX_PATH as usize] = [0; windows_sys::Win32::Foundation::MAX_PATH as usize];
                let required_length = windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW(
                    file_handle,
                    buffer.as_mut_ptr(),
                    windows_sys::Win32::Foundation::MAX_PATH,
                    0
                );

                if  required_length > 0 && required_length <= windows_sys::Win32::Foundation::MAX_PATH {
                    let file_path = crate::utils::convert::lpwstr_2_string(buffer.as_ptr());
                    crate::log!(trace, "nt_query_directory_file by api: file_path: {:?} {:?}", file_handle, file_path);
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
    file_handle:         *mut win::Foundation::HANDLE,
    access_mask:         win::Storage::FileSystem::FILE_ACCESS_RIGHTS,
    object_attributes:   *mut windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES,
    io_status_block:     *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
    allocation_size:     *const i64,
    file_attributes:     win::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    share_access:        win::Storage::FileSystem::FILE_SHARE_MODE,
    create_disposition:  windows_sys::Wdk::Storage::FileSystem::NTCREATEFILE_CREATE_DISPOSITION,
    create_options:      windows_sys::Wdk::Storage::FileSystem::NTCREATEFILE_CREATE_OPTIONS,
    ea_buffer:           *const core::ffi::c_void,
    ea_length:           u32
    ) -> win::Foundation::NTSTATUS {

    use std::os::windows::ffi::OsStrExt;
    let zw_create_file: extern "system" fn(
        filehandle: *mut win::Foundation::HANDLE,
        desiredaccess: u32,
        objectattributes: *mut windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES,
        iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
        allocationsize: *const i64,
        fileattributes: u32,
        shareaccess: u32,
        createdisposition: u32,
        createoptions: u32,
        eabuffer: *const core::ffi::c_void,
        ealength: u32,
    ) -> win::Foundation::NTSTATUS = std::mem::transmute(NT_CREATE_FILE);
    
    let mut skip = false;
    if file_attributes == 0 && share_access == 0 { //pipe
        skip = true;
    }
    else if (file_attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_TEMPORARY) != 0 { //temporary
        skip = true;
    }
    else if file_attributes == 0 && share_access & windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE == 0 {
        skip = true;
    }

    if !skip && !object_attributes.is_null() {

        let object_name = (*object_attributes).ObjectName;
        if !object_name.is_null() {

            let buffer = (*object_name).Buffer;
            let length = (*object_name).Length;

            let mut rtype = crate::replace::ReplaceType::Unknown;
            let mut is_mount_point_manager = false;
            if access_mask & win::Storage::FileSystem::FILE_LIST_DIRECTORY != 0 {
                rtype = crate::replace::ReplaceType::Dir;
            }
            else {
                rtype = crate::replace::ReplaceType::File;

                // UTF-16 literal for "\\??\\MountPointManager"
                const MOUNT_POINT_MANAGER_UTF16: &[u16] = &[
                    b'\\' as u16, b'?' as u16, b'?' as u16, b'\\' as u16,
                    b'M' as u16, b'o' as u16, b'u' as u16, b'n' as u16, b't' as u16,
                    b'P' as u16, b'o' as u16, b'i' as u16, b'n' as u16, b't' as u16,
                    b'M' as u16, b'a' as u16, b'n' as u16, b'a' as u16, b'g' as u16, b'e' as u16, b'r' as u16,
                ];
                const MOUNT_POINT_MANAGER_BYTE_LEN: u16 = (MOUNT_POINT_MANAGER_UTF16.len() * 2) as u16;
                
                is_mount_point_manager = if !buffer.is_null() && length == MOUNT_POINT_MANAGER_BYTE_LEN {
                    if rtype == crate::replace::ReplaceType::File {
                        let slice = std::slice::from_raw_parts(buffer, MOUNT_POINT_MANAGER_UTF16.len());
                        slice == MOUNT_POINT_MANAGER_UTF16
                    }
                    else {
                        false
                    }
                } 
                else {
                    false
                };
            }

            if !buffer.is_null() && length > 0 && !is_mount_point_manager {

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
                crate::log!(trace, "nt_create_file hook path: {} - {}", if rtype == crate::replace::ReplaceType::Dir { "dir" } else { "file" }, name);
                let replace = crate::replace::nt_replace(&mut name, rtype);
                if replace == crate::replace::ReplaceNtResult::Success {
                    //TODO elpase 10ms, need optimize. 
                    crate::log!(trace, "nt_create_file replace hook: {}", name.clone());

                    let mut object_name_source_wide_char = std::ffi::OsString::from(name.clone()).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
                    /*
                    let mut object_name: windows_sys::Win32::Foundation::UNICODE_STRING = std::mem::zeroed();
                    let ret = windows_sys::Wdk::Storage::FileSystem::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
                    if ret != windows_sys::Win32::Foundation::STATUS_SUCCESS {
                        crate::log!(error, "rtl init unicode string failed.");
                    }
                     
                    let mut fake_obejct_name_adapter = windows_sys::Win32::Foundation::UNICODE_STRING {
                       Length: object_name.Length,
                       MaximumLength: object_name.MaximumLength,
                       Buffer: object_name.Buffer,
                    };
                    */
                    let mut fake_obejct_name_adapter = windows_sys::Win32::Foundation::UNICODE_STRING {
                       Length: object_name_source_wide_char.len().saturating_sub(1) as u16 * 2,
                       MaximumLength: object_name_source_wide_char.len() as u16 * 2,
                       Buffer: object_name_source_wide_char.as_mut_ptr(),
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

                    if nt_status == win::Foundation::STATUS_SUCCESS {
                        NT_HANDLE_AND_DIR.with(|cell| {
                            cell.borrow_mut().insert(*file_handle as windows_sys::Win32::Foundation::HANDLE, name);
                        });
                    }
                    else {
                        if nt_status == win::Foundation::STATUS_OBJECT_NAME_NOT_FOUND {
                            crate::log!(error, "zw_create_file failed! object name not found path: {}", name);
                        }
                        else {
                            crate::log!(error, "zw_create_file failed! error_code: {:#X} path: {}", nt_status, name);

                            if nt_status == win::Foundation::STATUS_SHARING_VIOLATION {
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
                else if let crate::replace::ReplaceNtResult::VirtualIncludesDir(unmodified) = replace {
                    crate::log!(trace, "nt_create_file includes hook replace path: {} {}", name.clone(), unmodified);

                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let mut args =  std::collections::HashMap::<String, String>::new();
                    args.insert("objectname".to_string(), unmodified.clone());
                    args.insert("replace".to_string(), name.clone());
                    args.insert("exists".to_string(), "".to_string());
                    
                    let syscall = {
                        let mut cid = SYS_CALL_ID.lock().unwrap();
                        let syscall = crate::syscallredirect::MirrorSysCall {
                            cid: *cid,
                            api: "NtCreateFile".into(),
                            args,
                            responder: tx,
                        };
                        *cid += 1;
                        syscall
                    };
                        
                    crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(syscall).unwrap();
                    let exists = rx.blocking_recv().unwrap();
                    if let Some(exists) = exists.get("exists") {
                        if exists == "true" {
                            let mut object_name_source_wide_char = std::ffi::OsString::from(name.clone()).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
                            let mut fake_obejct_name_adapter = windows_sys::Win32::Foundation::UNICODE_STRING {
                                Length: object_name_source_wide_char.len().saturating_sub(1) as u16 * 2,
                                MaximumLength: object_name_source_wide_char.len() as u16 * 2,
                                Buffer: object_name_source_wide_char.as_mut_ptr(),
                            };
                            (*object_attributes).ObjectName = &mut fake_obejct_name_adapter;

                            let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block, allocation_size,
                                file_attributes, share_access, windows_sys::Wdk::Storage::FileSystem::FILE_OPEN_IF, create_options, ea_buffer, ea_length
                            );
                    
                            if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS {
                                crate::log!(error, "zw_create_file includes dir success! path: {} handle: {:?}", unmodified, *file_handle);

                                NT_HANDLE_AND_DIR.with(|cell| {
                                    cell.borrow_mut().insert(*file_handle as windows_sys::Win32::Foundation::HANDLE, unmodified);
                                });
                            }
                            else {
                                crate::log!(error, "zw_create_file includes dir failed!: path: {} handle: {:?} status: {:#X}", name, *file_handle, nt_status);
                            }
                            return nt_status;
                        }
                        else
                        {
                            crate::log!(trace, "nt_create_file includes dir file not exists: {}", unmodified);
                            return windows_sys::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND;
                        }
                    }
                    else {
                        crate::log!(trace, "nt_create_file includes dir file not exists: {}", unmodified);
                        return windows_sys::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND;
                    }
                }
                else if let crate::replace::ReplaceNtResult::NeedObtain(unmodified) = replace {
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    let mut args =  std::collections::HashMap::<String, String>::new();
                    args.insert("objectname".to_string(), unmodified.clone());
                    args.insert("expect".to_string(), name.clone());

                    let item = {
                        let guard = INCLUDES_CACHE.lock().unwrap();
                        guard.get(&unmodified).cloned()
                    };

                    if let Some(expect) = item {
                        crate::log!(trace, "nt_create_file redirect file handle path by cache expect: {}", expect);
                        
                        //let expect = format!(r"\??\{}", expect);
                        let mut object_name_source_wide_char = std::ffi::OsString::from(&expect).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

                        let mut expect_obejct_name_adapter = windows_sys::Win32::Foundation::UNICODE_STRING {
                            Length: object_name_source_wide_char.len().saturating_sub(1) as u16 * 2,
                            MaximumLength: object_name_source_wide_char.len() as u16 * 2,
                            Buffer: object_name_source_wide_char.as_mut_ptr(),
                        };
            
                        (*object_attributes).ObjectName = &mut expect_obejct_name_adapter;
                        let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                            allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                        );

                        if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS {
                    
                        }
                        else {
                            if nt_status == windows_sys::Win32::Foundation::STATUS_SHARING_VIOLATION {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                                    allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                                );
                                if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS {
                                   
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
                            let mut cid = SYS_CALL_ID.lock().unwrap();
                            let syscall = crate::syscallredirect::MirrorSysCall {
                                cid: *cid,
                                api: "NtCreateFile".into(),
                                args,
                                responder: tx,
                            };
                            *cid += 1;
                            syscall
                        };

                        let now = std::time::Instant::now();
                        crate::syscallredirect::REDIRECT_SYS_CALL_CHANNEL.tx.try_send(syscall).unwrap();
                        let expects = rx.blocking_recv().unwrap();
                        //let expects = std::collections::HashMap::<String, String>::new();
                        crate::log!(trace, "nt_create_file redirect file handle path by sync result: {} elapsed: {:?}", unmodified, now.elapsed());
                        if let Some(expect) = expects.get("expect") {
                            
                            INCLUDES_CACHE.lock().unwrap().insert(unmodified, expect.clone());

                            crate::log!(trace, "nt_create_file redirect file handle path by sync expect: {}", expect);
                            
                            //let expect = format!(r"\??\{}", expect);
                            let mut object_name_source_wide_char = std::ffi::OsString::from(&expect).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
                            let mut expect_obejct_name_adapter = windows_sys::Win32::Foundation::UNICODE_STRING {
                                Length: object_name_source_wide_char.len().saturating_sub(1) as u16 * 2,
                                MaximumLength: object_name_source_wide_char.len() as u16 * 2,
                                Buffer: object_name_source_wide_char.as_mut_ptr(),
                            };

                            (*object_attributes).ObjectName = &mut expect_obejct_name_adapter;

                            let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                                allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                            );

                            if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS {
                        
                            }
                            else {
                                if nt_status == windows_sys::Win32::Foundation::STATUS_SHARING_VIOLATION {
                                    std::thread::sleep(std::time::Duration::from_millis(5));
                                    let nt_status = zw_create_file(file_handle, access_mask, object_attributes, io_status_block,
                                        allocation_size, file_attributes, share_access, create_disposition, create_options, ea_buffer, ea_length
                                    );
                                    if nt_status == windows_sys::Win32::Foundation::STATUS_SUCCESS {
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
                                else
                                {
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
                    crate::log!(trace, "nt_create_file no need replace hook handle: {:?} {}", file_handle, name);
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

pub unsafe fn pass_project_and_replica_to_redriect(handle: win::Foundation::HANDLE, solution: &str, project: &str, replica: &str) {
    if !project.is_empty() {
        let arg = format!("solution:{}\nproject:{}\nreplica:{}\n", solution, project, replica);
        let mut bytes: u32 = 0;
        let mut overlapped: win::System::IO::OVERLAPPED = std::mem::zeroed();
        let ret = win::Storage::FileSystem::WriteFile(
            handle,
            arg.as_bytes().as_ptr(),
            arg.len() as u32,
            &mut bytes,
            &mut overlapped
        );
    
        if ret == win::Foundation::FALSE || bytes == 0 {
            let error = win::Foundation::GetLastError();
            crate::log!(error, "write pipe error, failed code: {}", error);
        }
        else {
            //FlushFileBuffers(handle);
            crate::log!(trace, "childprocess send message by pipe {} {:?}", if bytes > 0 {"success."} else {"failed."}, arg);
        }
    }
    else {
        crate::log!(warn, "don't pass project name and project path, use current path and don't redirect.");
    }
    win::Foundation::CloseHandle(handle);
}

pub unsafe fn nt_query_information_file(
    filehandle: windows_sys::Win32::Foundation::HANDLE,
    iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
    fileinformation: *mut core::ffi::c_void,
    length: u32,
    fileinformationclass: windows_sys::Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS) -> windows_sys::Win32::Foundation::NTSTATUS {

    crate::log!(trace, "nt_query_information_file called");

    let nt_query_information_file: extern "system" fn(
        filehandle: windows_sys::Win32::Foundation::HANDLE,
        iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
        fileinformation: *mut core::ffi::c_void,
        length: u32,
        fileinformationclass: windows_sys::Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS
    ) -> windows_sys::Win32::Foundation::NTSTATUS = std::mem::transmute(NT_QUERY_INFORMATION_FILE);

    let skip = NT_HANDLE_AND_DIR.with(|cell| {
        let handle_and_dir = cell.borrow();
        return handle_and_dir.get(&filehandle).is_some();
    });

    if false && skip && fileinformationclass == windows_sys::Wdk::Storage::FileSystem::FileIsRemoteDeviceInformation {
        if !fileinformation.is_null() {
            *(fileinformation as *mut u8) = 0;
        }

        if !iostatusblock.is_null() {
            (*iostatusblock).Anonymous.Status = windows_sys::Win32::Foundation::STATUS_SUCCESS;
            (*iostatusblock).Information = 1 as _;
        }
        return windows_sys::Win32::Foundation::STATUS_SUCCESS;
    }
    else {
        let nt_status = nt_query_information_file(
            filehandle,
            iostatusblock,
            fileinformation,
            length,
            fileinformationclass
        );

        return nt_status;
    }
}

pub unsafe fn nt_query_volume_information_file(
    filehandle: windows_sys::Win32::Foundation::HANDLE,
    iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
    fsinformation: *mut core::ffi::c_void,
    length: u32,
    fsinformationclass: windows_sys::Wdk::Storage::FileSystem::FS_INFORMATION_CLASS
) -> windows_sys::Win32::Foundation::NTSTATUS {

    //log!(trace, "nt_query_volume_info_file called");
    
    if filehandle.is_null() {
        return windows_sys::Win32::Foundation::STATUS_INVALID_HANDLE;
    }

    crate::logger::output_debug_string(&format!("nt_query_volume_information_file called, handle: {:?}, class: {:?}" ,filehandle, fsinformationclass));

    let skip = NT_HANDLE_AND_DIR.try_with(|cell| {
        let handle_and_dir = cell.try_borrow();
        match handle_and_dir {
            Ok(handle_and_dir) => {
                handle_and_dir.get(&filehandle).is_some()
            },
            Err(_) => {
                false
            }
        }
    }).unwrap_or(false);
    
    if skip && fsinformationclass == windows_sys::Wdk::Storage::FileSystem::FileFsDeviceInformation {
        let file_fs_device_info = fsinformation as *mut windows_sys::Wdk::System::SystemServices::FILE_FS_DEVICE_INFORMATION;
        if !file_fs_device_info.is_null() {
            (*file_fs_device_info).DeviceType = windows_sys::Win32::Storage::FileSystem::FILE_DEVICE_DISK;
            (*file_fs_device_info).Characteristics = 0;
        }

        if !iostatusblock.is_null() {
            (*iostatusblock).Anonymous.Status = windows_sys::Win32::Foundation::STATUS_SUCCESS;
            (*iostatusblock).Information = std::mem::size_of::<windows_sys::Wdk::System::SystemServices::FILE_FS_DEVICE_INFORMATION>() as _;
        }
        return windows_sys::Win32::Foundation::STATUS_SUCCESS;
    }
    else {
        let nt_query_information_file_: extern "system" fn(
            filehandle: windows_sys::Win32::Foundation::HANDLE,
            iostatusblock: *mut windows_sys::Win32::System::IO::IO_STATUS_BLOCK,
            fsinformation: *mut core::ffi::c_void,
            length: u32,
            fsinformationclass: windows_sys::Wdk::Storage::FileSystem::FS_INFORMATION_CLASS
        ) -> windows_sys::Win32::Foundation::NTSTATUS = std::mem::transmute(NT_QUERY_VOLUME_INFORMATION_FILE);

        let nt_status= nt_query_information_file_(
            filehandle,
            iostatusblock,
            fsinformation,
            length,
            fsinformationclass
        );
        return nt_status;
    }
}

/* 
pub unsafe fn nt_query_full_attributes_file(
    objectattributes: windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES,
    fileinformation: *mut windows_sys::Wdk::Storage::FileSystem::FILE_NETWORK_OPEN_INFORMATION,
) -> windows_sys::Win32::Foundation::NTSTATUS {
    log!(trace, "nt_query_full_attributes_file called");
    return windows_sys::Win32::Foundation::STATUS_SUCCESS;
}
*/

pub unsafe fn nt_close(handle: windows_sys::Win32::Foundation::HANDLE) -> windows_sys::Win32::Foundation::NTSTATUS {

    let nt_close_inner: extern "system" fn(
        handle: windows_sys::Win32::Foundation::HANDLE
    ) -> windows_sys::Win32::Foundation::NTSTATUS = std::mem::transmute(NT_CLOSE);

    //do not remove handle from map, when cl.exe run done, it will close all handles.
    //NT_HANDLE_MAP_FILES.write().unwrap().remove(&(handle as i32));

    let nt_status = nt_close_inner(handle);
    return nt_status;
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

    #[test]
    fn hex_func_test() {
        let val = *CALL_TEMPLATE;
        assert_eq!(val, 0x7462);
    }

}