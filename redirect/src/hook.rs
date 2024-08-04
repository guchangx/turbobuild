

pub unsafe fn init_hook() {

    println!("init hook functions");
    crate::functions::CREATE_FILE_A = winapi::um::fileapi::CreateFileA as *mut std::ffi::c_void;
    println!("crate::functions::CREATE_FILE_A: {:#?}", crate::functions::CREATE_FILE_A);
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A), crate::functions::create_file_a as _);

    crate::functions::CREATE_FILE_W = winapi::um::fileapi::CreateFileW as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W), crate::functions::create_file_w as _);

    let module = crate::utils::convert::string_2_lpstr("KernelBase.dll".to_string()).unwrap();

    let func_create_file_a = crate::utils::convert::string_2_lpstr("CreateFileA".to_string()).unwrap();
    let kernelbase_create_file_a = crate::detours::DetourFindFunction(module,  func_create_file_a);
    println!("kernelbase create file a:{:#?}", kernelbase_create_file_a);
    if kernelbase_create_file_a as usize == 0 {
        println!("can not find create_file_a in kernelbase module");
    }
    else {
        println!("find create_file_a in kernelbase module.");
        crate::functions::CREATE_FILE_A_KERNEL_BASE =  kernelbase_create_file_a;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A_KERNEL_BASE), crate::functions::kernelbase_create_file_a as _);   
    }

    let func_create_file_w = crate::utils::convert::string_2_lpstr("CreateFileW".to_string()).unwrap();
    let kernelbase_create_file_w = crate::detours::DetourFindFunction(module,  func_create_file_w);
    println!("kernelbase create file w:{:#?}", kernelbase_create_file_w);
    let module_w = crate::utils::convert::string_2_lpwstr("KernelBase.dll".to_string()).unwrap(); 
    let kernel_dll_handle = winapi::um::libloaderapi::GetModuleHandleW(module_w);
    println!("kernel_dll_handle:{:#?}", kernel_dll_handle);

    let raw_p = winapi::um::libloaderapi::GetProcAddress(kernel_dll_handle, func_create_file_w);
    println!("kernel raw_p:{:#?}", raw_p);
    
    if kernelbase_create_file_w as usize == 0 {
        println!("can not find create_file_a in kernelbase module");
    }
    else {
        println!("find create_file_w in kernelbase module.");
        crate::functions::CREATE_FILE_W_kERNEL_BASE =  kernelbase_create_file_w;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W_kERNEL_BASE), crate::functions::kernelbase_create_file_w as _);
    }
}