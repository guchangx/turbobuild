use windows_sys::Win32 as win;

pub unsafe fn init_hook() {
    crate::log!(info, "[{:?} {:?}] init compiler hk.", crate::SOLUTIONNAME.get(), crate::PROJECTNAME.get());

    crate::functions::CREATE_FILE_A = win::Storage::FileSystem::CreateFileA as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A), crate::functions::create_file_a as _);

    crate::functions::CREATE_FILE_W = win::Storage::FileSystem::CreateFileW as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W), crate::functions::create_file_w as _);

    //GetFileType
    crate::functions::GET_FILE_TYPE = win::Storage::FileSystem::GetFileType as *mut std::ffi::c_void;
    crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::GET_FILE_TYPE), crate::functions::get_file_type as _);

    let module = crate::utils::convert::string_2_lpstr("KernelBase.dll".to_string());

    let func_create_file_a = crate::utils::convert::string_2_lpstr("CreateFileA".to_string());
    let kernelbase_create_file_a = crate::detours::DetourFindFunction(module,  func_create_file_a);
    if kernelbase_create_file_a as usize == 0 {
        crate::log!(error, "can not find create_file_a in kernelbase module");
    }
    else {
        crate::functions::CREATE_FILE_A_KERNEL_BASE =  kernelbase_create_file_a;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_A_KERNEL_BASE), crate::functions::kernelbase_create_file_a as _);   
    }

    let func_create_file_w = crate::utils::convert::string_2_lpstr("CreateFileW".to_string());
    let kernelbase_create_file_w = crate::detours::DetourFindFunction(module,  func_create_file_w);
    
    if kernelbase_create_file_w as usize == 0 {
        crate::log!(error, "can not find create_file_a in kernelbase module");
    }
    else {
        crate::functions::CREATE_FILE_W_KERNEL_BASE =  kernelbase_create_file_w;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_FILE_W_KERNEL_BASE), crate::functions::kernelbase_create_file_w as _);
    }

    let func_create_process_a = crate::utils::convert::string_2_lpstr("CreateProcessA".to_string());
    let kernelbase_create_process_a = crate::detours::DetourFindFunction(module,  func_create_process_a);
    if kernelbase_create_process_a as usize == 0 {
        crate::log!(error, "can not find create_process_a in kernelbase module");
    }
    else {
        crate::functions::CREATE_PROCESS_A_KERNEL_BASE = kernelbase_create_process_a;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_PROCESS_A_KERNEL_BASE), crate::functions::kernelbase_create_process_a as _);
    }

    let func_create_process_w = crate::utils::convert::string_2_lpstr("CreateProcessW".to_string());
    let kernelbase_create_process_w = crate::detours::DetourFindFunction(module,  func_create_process_w);
    if kernelbase_create_process_w as usize == 0 {
        crate::log!(error, "can not find create_process_w in kernelbase module");
    }
    else {
        crate::functions::CREATE_PROCESS_W_KERNEL_BASE = kernelbase_create_process_w;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::CREATE_PROCESS_W_KERNEL_BASE), crate::functions::kernelbase_create_process_w as _);
    }

    let func_exit_process = crate::utils::convert::string_2_lpstr("ExitProcess".to_string());
    let kernelbase_exit_process = crate::detours::DetourFindFunction(module,  func_exit_process);
    if kernelbase_exit_process as usize == 0 {
        crate::log!(error, "can not find ExitProcess in kernelbase module");
    }
    else {
        crate::functions::EXIT_PROCESS_KERNELBASE =  kernelbase_exit_process;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::EXIT_PROCESS_KERNELBASE), crate::functions::kernelbase_exit_process as _);   
    }

    /* 
    let func_get_volume_information_by_handle_w = crate::utils::convert::string_2_lpstr("GetVolumeInformationByHandleW".to_string());
    let kernelbase_get_volume_information_by_handle_w = crate::detours::DetourFindFunction(module,  func_get_volume_information_by_handle_w);
    if kernelbase_get_volume_information_by_handle_w as usize == 0 {
        crate::log!(error, "can not find get_volume_information_by_handle_w in kernelbase module");
    }
    else {
        crate::functions::GET_VOLUME_INFORMATION_BY_HANDLE_W_KERNEL_BASE = kernelbase_get_volume_information_by_handle_w;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::GET_VOLUME_INFORMATION_BY_HANDLE_W_KERNEL_BASE), crate::functions::kernelbase_get_volume_information_by_handle_w as _);
    }

    let func_get_file_information_by_handle_ex = crate::utils::convert::string_2_lpstr("GetFileInformationByHandleEx".to_string());
    let kernelbase_get_file_information_by_handle_ex = crate::detours::DetourFindFunction(module,  func_get_file_information_by_handle_ex);
    if kernelbase_get_file_information_by_handle_ex as usize == 0 {
        crate::log!(error, "can not find get_file_information_by_handle_ex in kernelbase module");
    }
    else {
        crate::functions::GET_FILE_INFORMATION_BY_HANDLE_EX_KERNEL_BASE = kernelbase_get_file_information_by_handle_ex;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::GET_FILE_INFORMATION_BY_HANDLE_EX_KERNEL_BASE), crate::functions::kernelbase_get_file_information_by_handle_ex as _);
    }
    */

    let module = crate::utils::convert::string_2_lpstr("ntdll.dll".to_string());

    let func_nt_create_file = crate::utils::convert::string_2_lpstr("NtCreateFile".to_string());
    let nt_create_file = crate::detours::DetourFindFunction(module,  func_nt_create_file);
    if nt_create_file as usize == 0 {
        crate::log!(error, "can not find nt_create_file in nt module.");
    }
    else {
        crate::functions::NT_CREATE_FILE = nt_create_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_CREATE_FILE), crate::functions::nt_create_file as _);
    }

    
    let func_nt_query_directory_file = crate::utils::convert::string_2_lpstr("NtQueryDirectoryFile".to_string());
    let nt_query_directory_file = crate::detours::DetourFindFunction(module,  func_nt_query_directory_file);

    if nt_query_directory_file as usize == 0 {
        crate::log!(error, "can not find nt_query_directory_file in kernelbase module");
    }
    else {
        crate::functions::NT_QUERY_DIRECTORY_FILE = nt_query_directory_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_QUERY_DIRECTORY_FILE), crate::functions::nt_query_directory_file as _);
    }
    
    
    let func_nt_query_information_file = crate::utils::convert::string_2_lpstr("NtQueryInformationFile".to_string());
    let nt_query_information_file = crate::detours::DetourFindFunction(module,  func_nt_query_information_file);
    
    if nt_query_information_file as usize == 0 {
        crate::log!(error, "can not find nt_query_information_file in kernelbase module");
    }
    else {
        crate::functions::NT_QUERY_INFORMATION_FILE = nt_query_information_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_QUERY_INFORMATION_FILE), crate::functions::nt_query_information_file as _);
    }

    /*
    let func_nt_query_volume_information_file = crate::utils::convert::string_2_lpstr("NtQueryVolumeInformationFile".to_string());
    let nt_query_volume_information_file = crate::detours::DetourFindFunction(module,  func_nt_query_volume_information_file);

    if nt_query_volume_information_file as usize == 0 {
        crate::log!(error, "can not find nt_query_volume_information_file in kernelbase module");
    }
    else {
        crate::functions::NT_QUERY_VOLUME_INFORMATION_FILE = nt_query_volume_information_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_QUERY_VOLUME_INFORMATION_FILE), crate::functions::nt_query_volume_information_file as _);
    }

    let func_nt_query_full_attributes_file = crate::utils::convert::string_2_lpstr("NtQueryFullAttributesFile".to_string());
    let nt_query_full_attributes_file = crate::detours::DetourFindFunction(module,  func_nt_query_full_attributes_file);

    if nt_query_full_attributes_file as usize == 0 {
        crate::log!(error, "can not find nt_query_full_attributes_file in kernelbase module");
    }
    else {
        crate::functions::NT_QUERY_FULL_ATTRIBUTES_FILE = nt_query_full_attributes_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_QUERY_FULL_ATTRIBUTES_FILE), crate::functions::nt_query_full_attributes_file as _);
    }
    */

    let func_nt_close = crate::utils::convert::string_2_lpstr("NtClose".to_string());
    let nt_close = crate::detours::DetourFindFunction(module,  func_nt_close);

    if nt_close as usize == 0 {
        crate::log!(error, "can not find nt_close in kernelbase module");
    }
    else {
        crate::functions::NT_CLOSE = nt_close;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_CLOSE), crate::functions::nt_close as _);
    }

    
    let func_nt_write_file = crate::utils::convert::string_2_lpstr("NtWriteFile".to_string());
    let nt_write_file = crate::detours::DetourFindFunction(module,  func_nt_write_file);

    if nt_write_file as usize == 0 {
        crate::log!(error, "can not find nt_write_file in kernelbase module");
    }
    else {
        crate::functions::NT_WRITE_FILE = nt_write_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_WRITE_FILE), crate::functions::nt_write_file as _);
    }
     
    let func_nt_set_information_file = crate::utils::convert::string_2_lpstr("NtSetInformationFile".to_string());
    let nt_set_information_file = crate::detours::DetourFindFunction(module,  func_nt_set_information_file);

    if nt_set_information_file as usize == 0 {
        crate::log!(error, "can not find nt_set_information_file in kernelbase module");
    }
    else {
        crate::functions::NT_SET_INFORMATION_FILE = nt_set_information_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_SET_INFORMATION_FILE), crate::functions::nt_set_information_file as _);
    }
    
    let func_nt_read_file = crate::utils::convert::string_2_lpstr("NtReadFile".to_string());
    let nt_read_file = crate::detours::DetourFindFunction(module,  func_nt_read_file);
    if nt_read_file as usize == 0 {
        crate::log!(error, "can not find nt_read_file in kernelbase module");
    }
    else {
        crate::functions::NT_READ_FILE = nt_read_file;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NT_READ_FILE), crate::functions::nt_read_file as _);
    }

    /* 
    //NdrClientCall2
    let module = crate::utils::convert::string_2_lpstr("Rpcrt4.dll".to_string());
    let func_ndr_client_call2 = crate::utils::convert::string_2_lpstr("NdrClientCall2".to_string());
    let ndr_client_call2 = crate::detours::DetourFindFunction(module,  func_ndr_client_call2);

    if ndr_client_call2 as usize == 0 {
        crate::log!(error, "can not find ndr_client_call2 in rpcrt4 module");
    }
    else {
        crate::functions::NDR_CLIENT_CALL2 = ndr_client_call2;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::NDR_CLIENT_CALL2), crate::functions::ndr_client_call2 as _);
    }

    //I_RpcSendReceive
    let func_i_rpc_send_receive = crate::utils::convert::string_2_lpstr("I_RpcSendReceive".to_string());
    let i_rpc_send_receive = crate::detours::DetourFindFunction(module,  func_i_rpc_send_receive);

    if i_rpc_send_receive as usize == 0 {
        crate::log!(error, "can not find i_rpc_send_receive in rpcrt4 module");
    }
    else {
        crate::functions::I_RPC_SEND_RECEIVE = i_rpc_send_receive;
        crate::detours::DetourAttach(core::ptr::addr_of_mut!(crate::functions::I_RPC_SEND_RECEIVE), crate::functions::i_rpc_send_receive as _);
    }
    */
}