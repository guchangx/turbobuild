use winapi::um::winnt::EVENTLOG_SEQUENTIAL_READ;


pub async fn pre_sync_file(pre_sync_file: &crate::compiler::compiler::PreSyncFile) -> crate::compiler::compiler::PreSyncFile {
    
    let kind = pre_sync_file.sync_kind.to_str().unwrap();
    let mut toolchain_path = pre_sync_file.toolchain_path.clone().into_string().unwrap();
    let mut win_kits_path = pre_sync_file.windows_kits_path.clone().into_string().unwrap();
    if (!pre_sync_file.toolchain_path.is_empty() || !pre_sync_file.windows_kits_path.is_empty()) 
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

        let pre_sync_file = crate::compiler::compiler::PreSyncFile {
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
    return crate::compiler::compiler::PreSyncFile::default();
}

fn fetch_local_msvc_compiler(compiler_path: &str) -> Option<String> {
    //todo 初始化的时候就把路径读到内存中
    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
    let remote_compiler_path = std::path::Path::new(compiler_path);
    if remote_compiler_path.file_name() == Some(std::ffi::OsStr::new("cl.exe")) {
        let mut version = remote_compiler_path.into_iter()
            .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe");
        let compiler_version = version.nth(0).unwrap();

        let current_dir = std::env::current_dir().unwrap();
        let compiler_mapping_path = current_dir.join("FileCache").join("MSVC").join(compiler_version);
        if compiler_mapping_path.exists() {
            let arch = remote_compiler_path.components().nth_back(1).unwrap();
            let compiler_mapping_path = compiler_mapping_path.join(r"bin\Hostx64").join(arch).join("cl.exe");
            return Some(compiler_mapping_path.display().to_string());
        }
    }
    return None
}

fn fetch_local_windows_kits(path: &str) -> Option<String> {
    //C:\Program Files (x86)\Windows Kits\10\Include\10.0.22000.0\shared
    let path = std::path::PathBuf::from(path);
    let mut sdk_version = path.into_iter().filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap().ends_with(".0"));
    let sdk_version = sdk_version.nth(0).unwrap();
    
    let current_dir = std::env::current_dir().unwrap();
    let sdk_path = current_dir.join("FileCache").join("Windows Kits").join(sdk_version);
    if sdk_path.exists() {
        return Some(sdk_path.display().to_string())
    }
    return None;
}

pub async fn sync_file_to_local(multipart: &mut axum::extract::multipart::Multipart) {

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().expect("fetch name from multipart/form-data failed.").to_string();
        let file_name = field.file_name().expect("fetch file_name from multipart/form-data failed.").to_string();

        if name.contains("msvc bin") {
            let data = field.bytes().await.unwrap();
            if file_name.ends_with(".zip") {
            }
            else {
                let msvc_mapping_path = msvc_bin_mapping_path(file_name);
                

                let _ = std::fs::write(msvc_mapping_path, data);
            }
        }
        else if name.contains("msvc include") {
            if file_name.ends_with(".zip") {
                let s = msvc_include_mapping_path(file_name, true);
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
}

fn msvc_bin_mapping_path(file_path: String) -> std::path::PathBuf {
    let path = std::path::Path::new(&file_path);
    let compiler_version = path.into_iter()
        .filter(|arg| arg.to_str().unwrap().contains(".") || arg.to_str().unwrap() != "cl.exe")
        .nth(0).unwrap();

    let current_dir = std::env::current_dir().unwrap();
    let arch = path.components().nth_back(1).unwrap();
    let compiler_mapping_path = current_dir.join("FileCache").join("MSVC").join(compiler_version).join(r"bin\Hostx64")
    .join(arch).join("cl.exe");

    return compiler_mapping_path;
}

fn msvc_include_mapping_path(file_path: String, zip: bool) -> std::path::PathBuf {
    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\include
    let path = std::path::PathBuf::from(&file_path);
    let mut compiler_version: std::ffi::OsString;

    if zip {
        let version = path.into_iter()
            .filter(|arg| arg.to_str().unwrap().contains(".") || arg.to_str().unwrap() != ".zip")
            .nth(0)
            .unwrap()
            .to_owned();

        compiler_version = version;
    }
    else {
        let version = path.into_iter()
            .filter(|arg| arg.to_str().unwrap().contains(".") || arg.to_str().unwrap() != "cl.exe")
            .nth(0)
            .unwrap()
            .to_owned();

        compiler_version = version;
    }

    let include_mapping_path = std::env::current_dir().unwrap()
            .join("FileCache").join("MSVC").join(compiler_version).join("include");

    return include_mapping_path;

}

fn windows_kits_mapping_path(file_path: String) -> std::path::PathBuf {
    //C:\Program Files (x86)\Windows Kits\10\Include\10.0.22000.0\cppwint

    let path = std::path::PathBuf::from(&file_path);
    let win_kits_version = path.into_iter()
        .filter(|arg| arg.to_str().unwrap().contains(".") || arg.to_str().unwrap().ends_with(".0"))
        .nth(0)
        .unwrap();
    let include_mapping_path = std::env::current_dir().unwrap()
        .join("FileCache").join("Win Kits").join("10").join("Include").join(win_kits_version);
    return include_mapping_path;
}

fn unzip(path: &str, target: &str) {
    let source_zip = std::fs::File::open(path).unwrap();
    let mut zip_archive = zip::ZipArchive::new(source_zip).unwrap();
    zip_archive.extract(target).unwrap();
}
