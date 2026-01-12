use std::os::windows::ffi::OsStrExt;
use std::fmt::Write;

use windows_sys::Win32 as win;

pub fn route_file_system_operation(redirect: crate::communicate::package::pack::RemoteSyscall) -> crate::communicate::package::pack::LocalSyscall {
    let mut params = redirect.params.iter().map(|(param)| (param.key.clone(), param.value.clone())).collect::<std::collections::HashMap<String, String>>();
    match redirect.api.as_str() {
        "NtQueryDirectoryFile" => {
            let results = unsafe { redirect_nt_query_directory_file(params) };
            let local = crate::communicate::package::pack::LocalSyscall {
                cid: redirect.cid,
                api: redirect.api,
                params: results.iter().map(|(k, v)| crate::communicate::package::pack::Params {
                    key: k.clone(),
                    value: v.clone(),
                }).collect(),
                files: Vec::new(),
            };
            log::debug!("redirect net query directory file result: {:?}", local);
            return local;
        },
        "NtCreateFile" => {
            let context = unsafe { redirect_nt_create_file(&mut params) };
            match context {
                Ok((expect, data)) => {
                    if expect == "exists" {
                        let local = crate::communicate::package::pack::LocalSyscall {
                            cid: redirect.cid,
                            api: redirect.api,
                            params: params.iter().map(|(k, v)| crate::communicate::package::pack::Params {
                                key: k.clone(),
                                value: v.clone(),
                            }).collect(),
                            files: Vec::new(),
                        };

                        log::debug!("redirect nt create file result: {:?} exists {:?}", params, String::from_utf8(data));
                        return local;
                    }
                    else {
                        let local = crate::communicate::package::pack::LocalSyscall {
                            cid: redirect.cid,
                            api: redirect.api,
                            params: params.iter().map(|(k, v)| crate::communicate::package::pack::Params {
                                key: k.clone(),
                                value: v.clone(),
                            }).collect(),
                            files: vec![crate::communicate::package::pack::IntermediateResult{file: expect.clone(), content: data.clone()}],
                        };

                        log::debug!("redirect nt create file result: {:?} expect: {}", params, expect);
                        return local;
                    }   
                },
                Err(err) => {
                    let local = crate::communicate::package::pack::LocalSyscall {
                        cid: redirect.cid,
                        api: redirect.api,
                        params: vec![crate::communicate::package::pack::Params {
                            key: "error".to_string(),
                            value: format!("{}", err),
                        }],
                        files: Vec::new(),
                    };
                    log::debug!("redirect nt create file failed: {:?}", local);
                    return local;
                }
            }
        },
        "CreateFileW" => {
            let context = unsafe { redirect_create_file_w(&params) };
            match context {
                Ok((expect, data)) => {
                    let local = crate::communicate::package::pack::LocalSyscall {
                        cid: redirect.cid,
                        api: redirect.api,
                        params: params.iter().map(|(k, v)| crate::communicate::package::pack::Params {
                            key: k.clone(),
                            value: v.clone(),
                        }).collect(),
                        files: vec![crate::communicate::package::pack::IntermediateResult{file: expect.clone(), content: data}],
                    };
                    log::debug!("redirect create file w result: {:?} expect: {}", params, expect);
                    return local;
                },
                Err(err) => {
                    let local = crate::communicate::package::pack::LocalSyscall {
                        cid: redirect.cid,
                        api: redirect.api,
                        params: vec![crate::communicate::package::pack::Params {
                            key: "error".to_string(),
                            value: format!("{}", err),
                        }],
                        files: Vec::new(),
                    };
                    log::debug!("redirect create file w failed: {:?}", local);
                    return local;
                }
            }
        },
        _ => {
            log::warn!("Received unknown file system operation from remote redirect: {:?}", redirect);
            let local = crate::communicate::package::pack::LocalSyscall {
                cid: redirect.cid,
                api: redirect.api,
                params: Vec::new(),
                files: Vec::new(),
            };
            return local;
        }
    }
}

unsafe fn redirect_nt_query_directory_file(params: std::collections::HashMap<String, String>) -> std::collections::HashMap<String, String> {

    let mut fileh = params.get("filehandle").unwrap().to_owned();
    if fileh.is_empty() {
        log::error!("filehandle parameter is empty");
        return std::collections::HashMap::new();
    }
    else if fileh.starts_with("\\\\?\\") {
        fileh = fileh.replace("\\\\?\\", "\\??\\");
    }
    else if !fileh.starts_with("\\??\\") {
        fileh = format!("\\??\\{}", fileh);
    }

    let mut object_name: win::Foundation::UNICODE_STRING = std::mem::zeroed();
    let object_name_source_wide_char = std::ffi::OsString::from(&fileh).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();

    let ret = windows_sys::Wdk::Storage::FileSystem::RtlInitUnicodeStringEx(&mut object_name, object_name_source_wide_char.as_ptr());
    if ret != win::Foundation::STATUS_SUCCESS {
        log::error!("failed to init object name: {} {:#x}", fileh, ret);
        return std::collections::HashMap::new();
    }

    let mut filehandle: win::Foundation::HANDLE = std::ptr::null_mut();
    let mut objectattributes: windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES = windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: std::ptr::null_mut(),
        ObjectName: &mut object_name,
        Attributes: win::Foundation::OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: std::ptr::null_mut(),
        SecurityQualityOfService: std::ptr::null_mut(),
    };

    let mut iostatusblock: win::System::IO::IO_STATUS_BLOCK = std::mem::zeroed();

    let nt_status = windows_sys::Wdk::Storage::FileSystem::NtCreateFile(
        &mut filehandle,
        win::Storage::FileSystem::FILE_LIST_DIRECTORY | win::Storage::FileSystem::SYNCHRONIZE,
        &mut objectattributes,
        &mut iostatusblock,
        std::ptr::null_mut(),
        win::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        win::Storage::FileSystem::FILE_SHARE_READ | win::Storage::FileSystem::FILE_SHARE_WRITE,
        windows_sys::Wdk::Storage::FileSystem::FILE_OPEN,
        windows_sys::Wdk::Storage::FileSystem::FILE_DIRECTORY_FILE | windows_sys::Wdk::Storage::FileSystem::FILE_SYNCHRONOUS_IO_NONALERT,
        std::ptr::null_mut(),
        0,
    );
    
    if nt_status != win::Foundation::STATUS_SUCCESS {
        log::error!("failed to open directory: {} with status: {:#x}", fileh, nt_status);
        return std::collections::HashMap::new();
    }

    if filehandle.is_null() {
        log::error!("failed to open directory: {}", fileh);
        return std::collections::HashMap::new();
    }
    else {
        let length = 65536;
        let mut buffer: Vec<u8> = vec![0; length];
        let fileinformation = buffer.as_mut_ptr() as *mut std::ffi::c_void;

        let fileinformationclass = windows_sys::Wdk::Storage::FileSystem::FileDirectoryInformation;
        let nt_status = windows_sys::Wdk::Storage::FileSystem::NtQueryDirectoryFile(
            filehandle,
            std::ptr::null_mut(),
            win::System::IO::PIO_APC_ROUTINE::None,
            std::ptr::null_mut(),
            &mut iostatusblock,
            fileinformation,
            length as u32,
            fileinformationclass,
            false,
            std::ptr::null_mut(),
            false,
        );

        if nt_status != win::Foundation::STATUS_SUCCESS {
            log::error!("failed to query directory: {} with status: {}", fileh, nt_status);
            win::Foundation::CloseHandle(filehandle);
            return std::collections::HashMap::new();
        } else {
            let mut filenames = std::string::String::new();
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
                                writeln!(&mut filenames, "{}|{}", file_name_str, (*file_info).FileAttributes).unwrap();
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
            log::info!("successfully queried directory: {} entry count: {}", fileh, entry_count);
            win::Foundation::CloseHandle(filehandle);

            let mut results = std::collections::HashMap::new();
            results.insert("fileinformation".to_string(), filenames.clone());
            
            return results;
        }
    }
}

unsafe fn redirect_nt_create_file(params: &mut std::collections::HashMap<String, String>) -> std::io::Result<(String, Vec<u8>)> {

    let mut objectname = params.get("objectname").unwrap().to_owned();
    if objectname.is_empty() {
        log::error!("objectname parameter is empty");
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "objectname parameter is empty"));
    }
    else if objectname.starts_with("\\\\?\\") {
        objectname = objectname.replace("\\\\?\\", "");
    }

    if let Some(expect) = params.get("expect") {
        match std::fs::read(&objectname) {
            Ok(data) => {
                return Ok((expect.clone(), data));
            },
            Err(err) => {
                return Err(err);
            },
        }
    }
    else if let Some(value) = params.get_mut("exists") {
        let path = std::path::Path::new(&objectname);
        if path.exists() {
            *value = "true".to_string();
            return Ok(("exists".to_string(), "true".as_bytes().to_vec()));
        }
        else {
            *value = "false".to_string();
            return Ok(("exists".to_string(), "false".as_bytes().to_vec()));
        }
    }
    else {
        log::error!("neither expect nor exists parameter is provided");
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "neither expect nor exists parameter is provided"));

    }
}

unsafe fn redirect_create_file_w(params: &std::collections::HashMap<String, String>) -> std::io::Result<(String, Vec<u8>)> {
    let filename = params.get("filename").unwrap().to_owned();

    match std::fs::read(filename) {
        Ok(data) => {
            let expect = params.get("expect").unwrap().to_owned();
            return Ok((expect, data));
        },
        Err(err) => {
            return Err(err);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirect_nt_query_directory_file_test() {
        tools::logger::init_once_logger();
        
        println!("test redirect_nt_query_directory_file");
        let mut dir = std::env::current_dir().unwrap().to_str().unwrap().to_string();
        //dir = "\\\\?\\D:\\turbobuild\\target\\debug\\Replica".into();
        //dir = "\\??\\D:\\turbobuild\\target\\debug\\Replica".into();
        dir = "E:\\TestFuture\\GammaRay\\GammaRayTool\\build".into();

        println!("current dir: {}", dir);
        let mut params = std::collections::HashMap::new();
        params.insert("filehandle".to_string(), dir);

        unsafe { redirect_nt_query_directory_file(params) };
    }

    #[test]
    fn lines_test() {
        let mut lines = String::new();
        writeln!(&mut lines, "line 1").unwrap();
        writeln!(&mut lines, "line 2").unwrap();
        writeln!(&mut lines, "line 3").unwrap();
        let files = lines.lines();
        let files_count = files.clone().count();
        assert_eq!(files_count, 3);
    }
}