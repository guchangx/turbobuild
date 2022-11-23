use std::string;

use winapi::shared::winerror::ERROR_SXS_XML_E_MISSINGWHITESPACE;



pub async fn pre_sync_file(pre_sync_file: &crate::compiler::compiler::SyncData) -> crate::compiler::compiler::SyncData {
    
    let kind = pre_sync_file.sync_kind.to_str().unwrap();
    let mut toolchain_path = pre_sync_file.toolchain_path.clone().into_string().unwrap();
    let mut win_kits_path = pre_sync_file.windows_kits_path.clone().into_string().unwrap();
    println!("pre sync file {:?} {:?} {:?}",  kind, toolchain_path, win_kits_path);
    if (!toolchain_path.is_empty() || !win_kits_path.is_empty()) 
        && (kind.contains("kits") || kind.contains("msvc")) {
        match fetch_local_msvc_compiler(&toolchain_path) {
            Some(path) => {
                    toolchain_path = path;
                },
            None => {
                toolchain_path = String::new();
            }
        }
        match fetch_local_windows_kits(&win_kits_path) {
            Some(path) => {
                win_kits_path = path;
            }
            None => {
                win_kits_path = String::new();
            }
        }

        let pre_sync_file = crate::compiler::compiler::SyncData {
                sync_kind: std::ffi::OsString::new(),
                toolchain_path: std::ffi::OsString::from(toolchain_path),
                windows_kits_path: std::ffi::OsString::from(win_kits_path),
                file_path: std::ffi::OsString::new(),
                file_name: std::ffi::OsString::new(),
                digest: std::ffi::OsString::new(),
                is_exists: true,
        };

        return pre_sync_file;
    }
    else if pre_sync_file.sync_kind == "sourcefile" {

    }
    else {

    }
    return crate::compiler::compiler::SyncData::default();
}

fn fetch_local_msvc_compiler(compiler_path: &str) -> Option<String> {
    //todo 初始化的时候就把路径读到内存中
    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\
    let remote_compiler_dir = std::path::Path::new(compiler_path);

    let version = remote_compiler_dir.into_iter()
        .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe")
        .nth(0)
        .unwrap();

    let current_dir = std::env::current_dir().unwrap();
    let compiler_mapping_path = current_dir.join("FileCache").join("MSVC").join(version);
    if compiler_mapping_path.exists() {
        let mut arch = remote_compiler_dir.components().nth_back(0).unwrap().as_os_str().to_string_lossy();
        if arch.contains("64") || arch.contains("86") {

        }
        else {
            arch = remote_compiler_dir.components().nth_back(1).unwrap().as_os_str().to_string_lossy();
        }

        let compiler_mapping_path = compiler_mapping_path.join(r"bin\Hostx64").join(arch.to_mut()).join("cl.exe");
        if compiler_mapping_path.exists() {
            return Some(compiler_mapping_path.display().to_string());
        }
        else {
            println!("fetch loacl msvc compiler. {:?}", compiler_mapping_path);
        }
    }
    else {
        println!("fetch loacl msvc compiler. {:?}", compiler_mapping_path);
    }
    
    return None
}

fn fetch_local_windows_kits(path: &str) -> Option<String> {
    //C:\Program Files (x86)\Windows Kits\10\Include\10.0.22000.0\shared
    let path = std::path::PathBuf::from(path);
    let kits_version = path.into_iter().filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap().ends_with(".0"))
        .nth(0).unwrap();
    
    let kit_path = std::env::current_dir().unwrap()
        .join("FileCache").join("Win Kits").join("10").join("Include").join(kits_version);
    if kit_path.exists() {
        return Some(kit_path.display().to_string())
    }
    return None;
}

pub async fn sync_file_to_local(multipart: &mut axum::extract::multipart::Multipart) -> std::ffi::OsString {

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().expect("fetch name from multipart/form-data failed.").to_string();
        let file_name = field.file_name().expect("fetch file_name from multipart/form-data failed.").to_string();

        println!("mame {:?}", name);
        println!("file name {:?}", file_name);

        if name.contains("msvc") {
            let data = field.bytes().await.unwrap();
            if file_name.ends_with(".zip") {
                let cursor = std::io::Cursor::new(data);
                let mut zip_archive = zip::ZipArchive::new(cursor).unwrap();
                let msvc_mapping_dir = msvc_mapping_dir(file_name);
                zip_archive.extract(msvc_mapping_dir.clone()).unwrap();
                return msvc_mapping_dir.into_os_string();
            }
            else {
                
                let msvc_mapping_dir = msvc_mapping_dir(file_name).join("cl.exe");

                let result = std::fs::write(msvc_mapping_dir, data);
                match result {
                    Ok(_) => {},
                    Err(error) => {
                        println!("dist sync file write failed. {:?}", error);
                    }
                }
            }
        }
        else if name.contains("kits") {
            let data = field.bytes().await.unwrap();
            if file_name.ends_with(".zip") {
                let cursor = std::io::Cursor::new(data);
                let mut zip_archive = zip::ZipArchive::new(cursor).unwrap();
                let kits_include_path = windows_kits_mapping_path(file_name);
                zip_archive.extract(kits_include_path.clone()).unwrap();
                return kits_include_path.into_os_string()
            }
            else {

            }
        }
        else if name.contains("win kits")
        {

        }
        else if name.contains("source file") {

        }
        else if name.contains("include file") {

        }
        else if name.contains("resource") 
        {

        }
    }
    return std::ffi::OsString::new();
}

fn msvc_mapping_dir(file_path: String) -> std::path::PathBuf {
     //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\include
    let path = std::path::Path::new(&file_path);

    let compiler_version = path.into_iter()
        .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe")
        .collect::<Vec<&std::ffi::OsStr>>()
        .first()
        .unwrap()
        .to_owned();

    
    let current_dir = std::env::current_dir().unwrap();
    let name = path.file_stem().unwrap();
    if name.to_str().unwrap().contains("86") || name.to_str().unwrap().contains("64") {
        let compiler_mapping_path = current_dir.join("FileCache").join("MSVC").join(compiler_version).join(r"bin\Hostx64")
        .join(name);
        let _ = std::fs::create_dir_all(&compiler_mapping_path).expect("crate msvc compiler cl.exe dir failed.");
        return compiler_mapping_path;
    }
    else if name.to_string_lossy().contains("include") {
        let compiler_include_mapping_path = current_dir.join("FileCache").join("MSVC").join(compiler_version).join("include");
        let _ = std::fs::create_dir_all(&compiler_include_mapping_path).expect("crate msvc compiler cl.exe dir failed.");
        return compiler_include_mapping_path;
    }
    else {
        return current_dir.join("FileCache").join("unnamed");
    }
}

fn windows_kits_mapping_path(file_path: String) -> std::path::PathBuf {
    //C:\Program Files (x86)\Windows Kits\10\Include\10.0.22000.0\cppwint

    let path = std::path::PathBuf::from(&file_path);
    let win_kits_version = path.into_iter()
        .filter(|arg| arg.to_str().unwrap().contains(".") || arg.to_str().unwrap().ends_with(".0"))
        .nth(0)
        .unwrap();
    let mut kits_include_mapping_path = std::path::PathBuf::new();
    if  win_kits_version.to_string_lossy().contains(".zip") {
        let version = win_kits_version.to_string_lossy().strip_suffix(".zip").unwrap().to_owned();
        kits_include_mapping_path = std::env::current_dir().unwrap()
            .join("FileCache").join("Win Kits").join("10").join("Include").join(version);
    }
    else {
        kits_include_mapping_path = std::env::current_dir().unwrap()
            .join("FileCache").join("Win Kits").join("10").join("Include").join(win_kits_version);
    }

    let _ = std::fs::create_dir_all(&kits_include_mapping_path).expect("crate windows kits include dir failed.");
    return kits_include_mapping_path;
}

fn unzip(path: &str, target: &str) {
    let source_zip = std::fs::File::open(path).unwrap();
    let mut zip_archive = zip::ZipArchive::new(source_zip).unwrap();
    zip_archive.extract(target).unwrap();
}
