

pub unsafe fn init_hook() {

    println!("init hook");
    crate::functions::CREATE_FILE_A = winapi::um::fileapi::CreateFileA as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A), crate::functions::create_file_a as _);

    crate::functions::CREATE_FILE_W = winapi::um::fileapi::CreateFileW as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W), crate::functions::create_file_w as _);
}