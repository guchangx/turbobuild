
use winapi::{
    shared::{minwindef::DWORD, ntdef::LPCWSTR,},
    um::{minwinbase::LPSECURITY_ATTRIBUTES, winnt::{HANDLE, LPCSTR},}
};

pub static mut CREATE_FILE_A: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_A_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut ZW_QUERY_DIRECTORY_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_CREATE_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;


const PIPE_PREFIX_CONTENT_W: &[u16] = &['\\' as u16, '\\' as u16, '.' as u16, '\\' as u16, 'p' as u16, 'i' as u16, 'p' as u16, 'e' as u16, '\\' as u16];  //\\.\\pipe\\ or \\??\\pipe\\

pub unsafe fn start_with_pipe_w(lp_file_name: *const u16) -> bool {
    if lp_file_name.is_null() {
        return false;
    } 
    else {
        let len = PIPE_PREFIX_CONTENT_W.len() as isize;
        for i in 0..len {
            let uchar = *lp_file_name.offset(i as isize);
            if uchar != PIPE_PREFIX_CONTENT_W[i as usize] && uchar == 0 {
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
        crate::log!(debug, "hook func create_file_a path: {}", path);

        let create_file_a: extern "C" fn(
            lp_file_name: LPCSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_A);

        let replace = crate::replace::replace(&mut path);
        if replace {
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
        let create_file_w_inner: extern "C" fn (
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

        crate::log!(debug, "hook func create_file_w, path: {}", path);

        let create_file_w: extern "C" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W);

        let replace = crate::replace::replace(&mut path);

        if replace {
            crate::log!(debug, "hook func create_file_w new, path: {}", path);
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

            if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
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
        
        crate::log!(debug, "hook func kernelbase_create_file_a, path: {}", path);

        let create_file_a: extern "C" fn(
            lp_file_name: LPCSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_A_KERNEL_BASE);

        let replace = crate::replace::replace(&mut path);

        if replace {
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
        let create_file_w_inner: extern "C" fn (
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

    if let Some(mut path) = option_path {

        if CREATE_FILE_W_KERNEL_BASE as usize == 0 {
            crate::log!(error, "can not find kernelbase create_file_w");
            return winapi::um::handleapi::INVALID_HANDLE_VALUE;
        }

        let create_file_w: extern "C" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W_KERNEL_BASE);

        crate::log!(debug, "hook func kernelbase_create_file_w, path: {}", path);

        let replace = crate::replace::replace(&mut path);
        if replace {
            crate::log!(debug, "hook func kernelbase_create_file_w new, path: {}", path);
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

pub unsafe fn zw_query_directory_file(
    file_handle: super::ntdef::types::HANDLE,
    event: super::ntdef::types::HANDLE,
    apc_routine: super::ntdef::types::PVOID,
    apc_context: super::ntdef::types::PVOID,
    io_status_block: super::ntdef::structs::PIO_STATUS_BLOCK,
    file_information: super::ntdef::types::PVOID,
    length: super::ntdef::types::ULONG,
    file_information_class: super::ntdef::types::FILE_INFORMATION_CLASS,
    return_single_entry: super::ntdef::types::BOOLEAN,
    file_name: super::ntdef::structs::PUNICODE_STRING,
    restart_scan: super::ntdef::types::BOOLEAN
    ) -> super::ntdef::types::NTSTATUS {

    let zw_query_directory_file: super::ntdef::functions::ZwQueryDirectoryFile = std::mem::transmute(ZW_QUERY_DIRECTORY_FILE);

    if !file_name.is_null() {
        let buffer = (*file_name).Buffer;
        let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();
        
        crate::log!(debug, "hook func zw_query_directory_file, path: {:?}", name);
    }

    let nt_status = zw_query_directory_file(
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
    return nt_status;
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

    use std::os::windows::ffi::{OsStringExt, OsStrExt};

    let zw_create_file: super::ntdef::functions::ZwCreateFile = std::mem::transmute(NT_CREATE_FILE);
    
    if !object_attributes.is_null() {
        let object_name = (*object_attributes).ObjectName;
        if !object_name.is_null() {
            let buffer = (*object_name).Buffer;
            let length = (*object_name).Length;
            if !buffer.is_null() && length > 0 {

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
                // up and down are 2 way to get string from utf16 slice.

                let mut name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();

                //crate::log!(debug, "nt_create_file hook path: {:?}", name);
                let replace = crate::replace::replace_dir(&mut name);
                if replace {
                    crate::log!(debug, "nt_create_file replace hook: {:?}", name.clone());

                    let mut object_name: winapi::shared::ntdef::UNICODE_STRING = std::mem::zeroed();

                    let object_name_source_wide_char = std::ffi::OsString::from(name.clone()).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

                    let ret = ntapi::ntrtl::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
                    if ret != winapi::shared::ntstatus::STATUS_SUCCESS {
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

                    if nt_status != winapi::shared::ntstatus::STATUS_SUCCESS {
                        crate::log!(error, "zw_create_file faile. status: {:?}", (*io_status_block).Status);
                    }
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
    }
}