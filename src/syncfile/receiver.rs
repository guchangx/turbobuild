
struct FileCahe {
    file_name: String,
    file_path: std::path::PathBuf,
    digest: String,
}

pub async fn pre_sync_file(pre_sync_file: &crate::compiler::compiler::PreSyncFile) -> crate::compiler::compiler::PreSyncFile {
    if pre_sync_file.file_kind == "toolchain" {
        if pre_sync_file.file_name == "cl.exe" {
            match fetch_local_msvc_compiler(pre_sync_file.file_path.to_str().unwrap()) {
                Some(path) => {
                    let pre_sync_file = crate::compiler::compiler::PreSyncFile {
                        file_kind: std::ffi::OsString::new(),
                        file_path: std::ffi::OsString::new(),
                        file_name: std::ffi::OsString::from("cl.exe"),
                        digest: std::ffi::OsString::new(),
                        is_exists: true,
                    };
                    return pre_sync_file;
                }
                None => {

                }
            }
        }
    }
    else if pre_sync_file.file_kind == "sourcefile" || pre_sync_file.file_kind == "includefile" {

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

pub async fn sync_file_to_local(multipart: &mut axum::extract::multipart::Multipart) {

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().expect("fetch name from multipart/form-data failed.").to_string();
        let file_name = field.file_name().expect("fetch file_name from multipart/form-data failed.").to_string();

        if name.contains("msvc bin") {
            let data = field.bytes().await.unwrap();
            let msvc_mapping_path = msvc_bin_mapping_path(file_name);
            let _ = std::fs::write(msvc_mapping_path, data);
        }
        else if name.contains("msvc include") {
            let s = msvc_include_mapping_path(file_name);

        }
        else if name.contains("win sdk")
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

fn msvc_include_mapping_path(file_path: String) -> std::path::PathBuf {
    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\include
    let mut path = std::path::PathBuf::from(&file_path);
    for _ in 0..4 {
        path.pop();
    };
    
    return path;
    
}