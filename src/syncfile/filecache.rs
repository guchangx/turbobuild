
struct FileCahe {
    file_name: String,
    file_path: std::path::PathBuf,
    digest: String,
}

pub async fn pre_sync_file(pre_sync_file: &crate::compiler::compiler::PreSyncFile) {
    if pre_sync_file.file_kind == "toolchain" {
        if pre_sync_file.file_name == "cl.exe" {
            match fetch_local_msvc_compiler(pre_sync_file.file_path.to_str().unwrap()) {
                Some(path) => {
                    println!("mapping file path: {:?}", path);
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
}

pub async fn sync_file_to_local(multipart: &mut axum::extract::multipart::Multipart) {

    while let Some(field) = multipart.next_field().await.unwrap() {
        let _name = field.name().expect("fetch name from multipart/form-data failed.").to_string();
        let file_name = field.file_name().expect("fetch file_name from multipart/form-data failed.").to_string();

        let data = field.bytes().await.unwrap();
        let file_path = std::path::Path::new(&file_name);
        let dir = file_path.parent().unwrap();

        if !dir.exists() {
            std::fs::create_dir(dir).unwrap();
        }

        let _ = std::fs::write(file_path, data);
    }
}

fn fetch_local_msvc_compiler(compiler_path: &str) -> Option<String> {

    //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
    let remote_compiler_path = std::path::Path::new(compiler_path);
    if remote_compiler_path.file_name() == Some(std::ffi::OsStr::new("cl.exe")) {
        let mut version = remote_compiler_path.into_iter()
            .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe");
        let compiler_version = version.nth(0).unwrap();

        let current_dir = std::env::current_dir().unwrap();
        let file_cache = current_dir.join("FileCache").join("MSVC").join(compiler_version);
        if file_cache.exists() {
            let arch = remote_compiler_path.components().nth_back(1).unwrap();
            let compiler_mapping_path = file_cache.join(r"bin\Hostx64").join(arch).join("cl.exe");
            return Some(compiler_mapping_path.display().to_string());
        }
    }
    return None
}