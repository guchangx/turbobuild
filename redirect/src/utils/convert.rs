use std::os::windows::ffi::OsStrExt;


pub fn lpwstr_2_string(lp_param: winapi::um::winnt::LPCWSTR) -> core::option::Option<std::string::String> {
    use std::os::windows::prelude::*;

    if !lp_param.is_null() {
        let mut result_string = "".to_string();
        let non_null_wstr_ptr = unsafe { std::ptr::NonNull::new_unchecked(lp_param as *mut _) };
        let wide_slice = unsafe { std::slice::from_raw_parts(non_null_wstr_ptr.as_ptr(), std::u16::MAX as usize) };
    
        if let Some(null_index) = wide_slice.iter().position(|&x| x == 0) {
          
            let truncated_slice = &wide_slice[..null_index];
            
            let os_string = std::ffi::OsString::from_wide(truncated_slice);
            
            result_string = os_string.to_string_lossy().into_owned();

        } else {
            println!("No null terminator found.");
        }
        return Some(result_string);   
    }
    else
    {
        return None;
    }
   
}

pub fn string_2_lpwstr(param: String) -> core::option::Option<winapi::um::winnt::LPWSTR> {
    use std::os::windows::prelude::*;
    if !param.is_empty() {

        let os_string = std::ffi::OsString::from(param);

        let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
        wchars.push(0);
        return Some(wchars.as_ptr() as winapi::um::winnt::LPWSTR);
    }
    return None;
} 