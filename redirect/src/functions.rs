
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
pub static mut CREATE_FILE_W_kERNEL_BASE: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;

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
            let option_fake_path = crate::utils::convert::string_2_lpwstr(path);
            if let Some(fake_path) = option_fake_path {
                
                let handle = create_file_w(
                    fake_path,
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
                return 0 as HANDLE;
            }
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

    println!("hook func kernelbase_create_file_w");
    let option_path = crate::utils::convert::lpwstr_2_string(lp_file_name);

    if let Some(path) = option_path {

        let create_file_w: extern "C" fn (
            lp_file_name: LPCWSTR,
            dw_desired_access: DWORD,
            dw_share_mode: DWORD,
            lp_security_attributes: LPSECURITY_ATTRIBUTES,
            dw_creation_disposition: DWORD,
            dw_flags_and_attributes: DWORD,
            h_template_file: HANDLE,
        ) -> HANDLE = std::mem::transmute(CREATE_FILE_W_kERNEL_BASE);

        println!("hook: {}", path);

        let path = crate::replace::replace(path);
        if !path.is_empty() {
            
            let option_fake_path = crate::utils::convert::string_2_lpwstr(path);
            if let Some(fake_path) = option_fake_path {
                let handle = create_file_w(
                    fake_path,
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
                return 0 as HANDLE;
            }
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
