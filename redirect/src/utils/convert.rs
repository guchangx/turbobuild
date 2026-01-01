use crate::detours::LPCWSTR;

pub fn lpwstr_2_string(lp_param: LPCWSTR) -> core::option::Option<std::string::String> {
    use std::os::windows::prelude::*;

    if !lp_param.is_null() {
        let non_null_wstr_ptr = unsafe { std::ptr::NonNull::new_unchecked(lp_param as *mut _) };
        
        let len = unsafe { (0..).take_while(|&i| *lp_param.offset(i) != 0).count() };
        if len != 0 {
            let wide_slice = unsafe { std::slice::from_raw_parts(non_null_wstr_ptr.as_ptr(), len) };
            let os_string = std::ffi::OsString::from_wide(wide_slice);
            let result_string = os_string.to_string_lossy().into_owned();
            return Some(result_string);
        }
        else {
            return None;
        }
    }
    else
    {
        return None;
    }
}

pub fn string_2_lpwstr(param: String) -> Vec<u16> {
    use std::os::windows::prelude::*;

    let os_string = std::ffi::OsString::from(param);

    let mut wchars = os_string.encode_wide().collect::<Vec<_>>();
    wchars.push(0);
    return wchars;
}
#[allow(unused_imports)]
pub fn os_string_2_lpwstr(os_string: std::ffi::OsString) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let wchars = os_string.encode_wide().chain(Some(0)).collect::<Vec<_>>();
    return wchars;
}

pub fn lpstr_2_string(lpstr: *const std::ffi::c_char) -> Result<String, std::str::Utf8Error> {
    unsafe {
        std::ffi::CStr::from_ptr(lpstr).to_str().map(String::from)
    }
}

pub fn string_2_lpstr(string: std::string::String) -> *mut std::ffi::c_char {
    let c_string = std::ffi::CString::new(string).expect("new CString from string failed.");
    return c_string.into_raw();
}