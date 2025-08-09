use std::os::windows::ffi::OsStrExt;

pub fn route_file_system_operation(redirect: crate::communicate::package::pack::RemoteRedirect) {
    match redirect.api.as_str() {
        "NtQueryDirectoryFile" => {

        },
        _ => {
            log::warn!("Received unknown file system operation from remote redirect: {:?}", redirect);
        }
    }
}

unsafe fn redirect_nt_query_directory_file(dir: String, params: std::collections::HashMap<String, String>) {
    
    let mut filehandle: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();

    let mut object_name: windows_sys::Win32::Foundation::UNICODE_STRING = std::mem::zeroed();
    let object_name_source_wide_char = std::ffi::OsString::from(&dir).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
    
    let ret = windows_sys::Wdk::Storage::FileSystem::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
    if ret != windows_sys::Win32::Foundation::STATUS_SUCCESS {
        log::error!("failed to init object name: {}", dir);
        return;
    }

    let mut objectattributes: windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES = windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: std::ptr::null_mut(),
        ObjectName: &mut object_name,
        Attributes: 0,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };

    let mut iostatusblock: windows_sys::Win32::System::IO::IO_STATUS_BLOCK = std::mem::zeroed();

    windows_sys::Wdk::Storage::FileSystem::NtCreateFile(
        &mut filehandle,
        windows_sys::Win32::Storage::FileSystem::FILE_LIST_DIRECTORY | windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE,
        &objectattributes,
        &mut iostatusblock,
        std::ptr::null_mut(),
        windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
        windows_sys::Win32::Storage::FileSystem::OPEN_EXISTING,
        windows_sys::Wdk::Storage::FileSystem::FILE_SYNCHRONOUS_IO_NONALERT,
        std::ptr::null_mut(),
        0,
    );

    if filehandle.is_null() {
        log::error!("failed to open directory: {}", dir);
        return;
    }
    else {
        let mut fileinformation = std::ptr::null_mut();
        let length = 1024; // Adjust buffer size as needed
        let fileinformationclass = windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation;
        let nt_status = windows_sys::Wdk::Storage::FileSystem::NtQueryDirectoryFile(
            filehandle,
            std::ptr::null_mut(),
            windows_sys::Win32::System::IO::PIO_APC_ROUTINE::None,
            std::ptr::null_mut(),
            &mut iostatusblock,
            &mut fileinformation as *mut _ as *mut std::ffi::c_void,
            length as u32,
            fileinformationclass,
            false,
            std::ptr::null_mut(),
            false,
        );
        if nt_status != windows_sys::Win32::Foundation::STATUS_SUCCESS {
            log::error!("failed to query directory: {} with status: {}", dir, nt_status);
        } else {

            let mut current_offset = 0usize;
            let mut entry_count = 0;
        
            loop {
                let current_entry = (fileinformation as *const u8).add(current_offset);
                entry_count += 1;
                match fileinformationclass {
                    windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation => {
                        let file_info = current_entry as *const windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_INFORMATION;
                        let file_name_length_bytes = (*file_info).FileNameLength as usize;

                        if file_name_length_bytes > 0 {
                            let file_name_slice = std::slice::from_raw_parts((*file_info).FileName.as_ptr(), file_name_length_bytes / 2);
                            if let Ok(file_name_str) = String::from_utf16(file_name_slice) {
                                log::info!("File: {} (Entry {})", file_name_str, entry_count);
                            } else {
                                log::warn!("failed to convert file name to UTF-16: {:?}", file_name_slice);
                            }
                        }

                        let next_entry_offset = (*file_info).NextEntryOffset;
                        if next_entry_offset == 0 {
                            break;
                        }
                        else {
                            current_offset += next_entry_offset as usize;
                        }
                    }
                    _ => {
                       
                    }
                }

                if current_offset >= length as usize {
                   
                    break;
                }
            }
            log::info!("successfully queried directory: {}", dir);
        }
    }

    windows_sys::Win32::Foundation::CloseHandle(filehandle);

}