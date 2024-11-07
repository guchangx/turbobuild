

pub unsafe fn init_hook() {

    println!("init hook functions");
    crate::functions::CREATE_FILE_A = winapi::um::fileapi::CreateFileA as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A), crate::functions::create_file_a as _);

    crate::functions::CREATE_FILE_W = winapi::um::fileapi::CreateFileW as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W), crate::functions::create_file_w as _);

    let module = crate::utils::convert::string_2_lpstr("KernelBase.dll".to_string()).unwrap();

    let func_create_file_a = crate::utils::convert::string_2_lpstr("CreateFileA".to_string()).unwrap();
    let kernelbase_create_file_a = crate::detours::DetourFindFunction(module,  func_create_file_a);
    if kernelbase_create_file_a as usize != 0 {
        println!("can not find create_file_a in kernelbase module");
    }
    else {
        println!("find create_file_a in kernelbase module.");
        crate::functions::CREATE_FILE_A_KERNEL_BASE =  kernelbase_create_file_a;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A_KERNEL_BASE), crate::functions::kernelbase_create_file_a as _);   
    }

    let func_create_file_w = crate::utils::convert::string_2_lpstr("CreateFileW".to_string()).unwrap();
    let kernelbase_create_file_w = crate::detours::DetourFindFunction(module,  func_create_file_w);
    
    if kernelbase_create_file_w as usize == 0 {
        println!("can not find create_file_a in kernelbase module");
    }
    else {
        println!("find create_file_w in kernelbase module.");
        crate::functions::CREATE_FILE_W_KERNEL_BASE =  kernelbase_create_file_w;
         
        let ret = crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W_KERNEL_BASE), crate::functions::kernelbase_create_file_w as _);
        if ret == winapi::shared::winerror::NO_ERROR as i32 {
            
        }
        else {
            println!("detour attach kernelbase_create_file_w failed. error code: {}", ret);
        }
        
    }

    let module = crate::utils::convert::string_2_lpstr("ntdll.dll".to_string()).unwrap();

    let func_nt_create_file = crate::utils::convert::string_2_lpstr("NtCreateFile".to_string()).unwrap();
    let nt_create_file = crate::detours::DetourFindFunction(module,  func_nt_create_file);
    if nt_create_file as usize == 0 {
        println!("can not find nt_create_file in nt module.");
    }
    else {
        println!("find nt_create_file in nt module.");
        crate::functions::NT_CREATE_FILE = nt_create_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_CREATE_FILE), crate::functions::nt_create_file as _);
    }

    let func_zw_query_directory_file = crate::utils::convert::string_2_lpstr("ZwQueryDirectoryFile".to_string()).unwrap();
    let zw_query_directory_file = crate::detours::DetourFindFunction(module,  func_zw_query_directory_file);
    
    if zw_query_directory_file as usize == 0 {
        println!("can not find zw_query_directory_file in kernelbase module");
    }
    else {
        println!("find zw_query_directory_file in nt module.");
        crate::functions::ZW_QUERY_DIRECTORY_FILE = zw_query_directory_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::ZW_QUERY_DIRECTORY_FILE), crate::functions::zw_query_directory_file as _);
    }
}