
use winapi::{
    shared::{
        minwindef::{DWORD, LPVOID},
        ntdef::LPCWSTR,
    },
    um::{
        minwinbase::LPSECURITY_ATTRIBUTES,
        winnt::{HANDLE, LPWSTR, LPCSTR},
    }
};

pub static mut CREATE_FILE_A: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
pub static mut CREATE_FILE_W: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;

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
        let fake_path = crate::replace::replace(path);
        if !fake_path.is_empty() {
            let fake_path = crate::utils::convert::string_2_lpstr(fake_path);
            if let Ok(path) = fake_path {
                
                let create_file_a: extern "C" fn(
                    lp_file_name: LPCSTR,
                    dw_desired_access: DWORD,
                    dw_share_mode: DWORD,
                    lp_security_attributes: LPSECURITY_ATTRIBUTES,
                    dw_creation_disposition: DWORD,
                    dw_flags_and_attributes: DWORD,
                    h_template_file: HANDLE,
                ) -> HANDLE = std::mem::transmute(CREATE_FILE_A);

                let handle = create_file_a(
                    path,
                    dw_desired_access,
                    dw_share_mode,
                    lp_security_attributes,
                    dw_creation_disposition,
                    dw_flags_and_attributes,
                    h_template_file,
                );

                unsafe {
                    let c_string = std::ffi::CString::from_raw(path);
                    drop(c_string);
                }

                return handle;
            }
            else {
                return 0 as HANDLE;
            }

        }
        else {
            return 0 as HANDLE;    
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
        let fake_path = crate::replace::replace(path);
        if !fake_path.is_empty() {
            let fake_path = crate::utils::convert::string_2_lpwstr(fake_path);
            if let Some(path) = fake_path {
                
                let create_file_w: extern "C" fn(
                    lp_file_name: LPCWSTR,
                    dw_desired_access: DWORD,
                    dw_share_mode: DWORD,
                    lp_security_attributes: LPSECURITY_ATTRIBUTES,
                    dw_creation_disposition: DWORD,
                    dw_flags_and_attributes: DWORD,
                    h_template_file: HANDLE,
                ) -> HANDLE = std::mem::transmute(CREATE_FILE_W);

                let handle = create_file_w(
                    path,
                    dw_desired_access,
                    dw_share_mode,
                    lp_security_attributes,
                    dw_creation_disposition,
                    dw_flags_and_attributes,
                    h_template_file,
                );
                return handle;
            }
            else {
                return 0 as HANDLE;
            }

        }
        else {
            return 0 as HANDLE;    
        }
        
    }
    else {
        return 0 as HANDLE;
    }

}