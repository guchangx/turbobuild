
use winapi::{
    shared::{
        minwindef::DWORD,
        ntdef::LPCWSTR,
    },
    um::{
        minwinbase::LPSECURITY_ATTRIBUTES,
        winnt::{HANDLE, LPCSTR},
    }
};

pub static mut CREATE_FILE_A: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_A_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W_KERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut ZW_QUERY_DIRECTORY_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut NT_CREATE_FILE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;

pub unsafe fn create_file_a(
    lp_file_name: LPCSTR,
    dw_desired_access: DWORD,
    dw_share_mode: DWORD,
    lp_security_attributes: LPSECURITY_ATTRIBUTES,
    dw_creation_disposition: DWORD,
    dw_flags_and_attributes: DWORD,
    h_template_file: HANDLE,
) -> HANDLE {

    println!("hook func create_file_a");
    let path = crate::utils::convert::lpstr_2_string(lp_file_name);
  
    if let Ok(path) = path {
        
        println!("hook: {}", path);

        let create_file_a: extern "C" fn(
            lp_file_name: LPCSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_A);

        let path = crate::replace::replace(path);
        if !path.is_empty() {
            let result_fake_path = crate::utils::convert::string_2_lpstr(path);
            if let Ok(fake_path) = result_fake_path {
                
                let handle = create_file_a(
                    fake_path,
                    dw_desired_access,
                    dw_share_mode,
                    lp_security_attributes,
                    dw_creation_disposition,
                    dw_flags_and_attributes,
                    h_template_file,
                );

                unsafe {
                    let c_string = std::ffi::CString::from_raw(fake_path);
                    drop(c_string);
                }
                if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    println!("kernelbase create_file_a failed! error_code: {}.", error_code);
                }

                return handle;
            }
            else {
                return 0 as HANDLE;
            }
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
                println!("create_file_a failed! error_code: {}.", error_code);
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

    println!("hook func create_file_w");
    let path = crate::utils::convert::lpwstr_2_string(lp_file_name);
    if let Some(path) = path {

        println!("hook: {}", path);

        let create_file_w: extern "C" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W);

        let path = crate::replace::replace(path.clone());

        if !path.is_empty() {
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
                println!("create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
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
                println!("create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
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

    println!("hook func kernelbase_create_file_a");
    let path = crate::utils::convert::lpstr_2_string(lp_file_name);
  
    if let Ok(path) = path {
        
        println!("hook: {}", path);

        let create_file_a: extern "C" fn(
            lp_file_name: LPCSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_A_KERNEL_BASE);

        let path = crate::replace::replace(path);

        if !path.is_empty() {
            let result_fake_path = crate::utils::convert::string_2_lpstr(path);
            if let Ok(fake_path) = result_fake_path {
                
                let handle = create_file_a(
                    fake_path,
                    dw_desired_access,
                    dw_share_mode,
                    lp_security_attributes,
                    dw_creation_disposition,
                    dw_flags_and_attributes,
                    h_template_file,
                );

                unsafe {
                    let c_string = std::ffi::CString::from_raw(fake_path);
                    drop(c_string);
                }

                if handle ==  winapi::um::handleapi::INVALID_HANDLE_VALUE {
                    let error_code = winapi::um::errhandlingapi::GetLastError();
                    println!("kernelbase create_file_a failed! error_code: {}.", error_code);
                }

                return handle;
            }
            else {
                return 0 as HANDLE;
            }
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
                println!("kernelbase create_file_a failed! error_code: {}.", error_code);
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
    return 0 as HANDLE;
    println!("hook func kernelbase_create_file_w");
    let option_path = crate::utils::convert::lpwstr_2_string(lp_file_name);

    if let Some(path) = option_path {

        if CREATE_FILE_W_kERNEL_BASE as usize == 0 {
            println!("can not find kernelbase create_file_w");
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

        println!("hook: {}", path);

        let path = crate::replace::replace(path);
        if !path.is_empty() {
            
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
                println!("kernelbase create_file_a failed! error_code: {} {:?}.", error_code, hook_path);
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
                println!("kernelbase create_file_w failed! error_code: {} {:?}.", error_code, hook_path);
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
        
        println!("hook func zw_query_directory_file");
        println!("hook: {:?}", name);
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
                println!("hook func nt_create_file");

                let utf16_slice = std::slice::from_raw_parts(buffer, (length / 2) as usize);
                let os_string = std::ffi::OsString::from_wide(utf16_slice);
                match os_string.into_string() {
                    Ok(string) => {
                        println!("object name length {} {:?}", length, string);
                    },
                    Err(_) => {
                        println!("can't convert osstring into string.");
                    }
                }

                let name = crate::utils::convert::lpwstr_2_string(buffer).unwrap();

                println!("nt_create_file hook: length {:?} {:?}", length, name);
                let fake_path = crate::replace::replace_dir(name);
                if !fake_path.is_empty() {
                    println!("nt_create_file replace hook: {:?}", fake_path.clone());

                    let mut object_name: winapi::shared::ntdef::UNICODE_STRING = std::mem::zeroed();

                    let object_name_source_wide_char = std::ffi::OsString::from(fake_path.clone()).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

                    let ret = ntapi::ntrtl::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
                    if ret != winapi::shared::ntstatus::STATUS_SUCCESS {
                        println!("init unicode string failed.");
                    }

                    let mut fake_obejct_name_adapter = crate::ntdef::structs::UNICODE_STRING {
                       Length: object_name.Length,
                       MaximumLength: object_name.MaximumLength,
                       Buffer: object_name.Buffer,
                    };

                    (*object_attributes).ObjectName = &mut fake_obejct_name_adapter;

                    println!("nt_create_file re hook: {:?}", crate::utils::convert::lpwstr_2_string((*(*object_attributes).ObjectName).Buffer).unwrap());
                    println!("nt_create_file re hook 1: {:?}", crate::utils::convert::lpwstr_2_string(object_name_source_wide_char.as_ptr()).unwrap());

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
                        println!("zw_create_file faile. status: {:?}", (*io_status_block).Status);
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