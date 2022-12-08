use std::io::Read;
use std::io::Write;

pub struct Sender<'a>{
    client: &'a crate::network::client::NetworkClient,
}

impl<'a> Sender<'a> {
    pub fn new(client: &'a crate::network::client::NetworkClient) -> Self {
        Sender {
            client,
        }
    }

    fn zip_dir(&self, dir: &str, name: &str, _client: &crate::network::client::NetworkClient) -> crate::compiler::compiler::SyncData {
        let mut path = std::path::PathBuf::from(dir);

        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Zstd);
        let start = std::time::Instant::now();
        self.zip_visit_dir(path.as_path(), path.as_path(), &mut zip, &options);
        let content = zip.finish().unwrap();
        let elapsed = start.elapsed();

        let content = std::borrow::Cow::from(content.get_ref());
        
        match path.extension() {
            Some(extension) => {
                let mut ex = extension.to_str().unwrap().to_string();
                ex.push_str(".zip");
                path.set_extension(ex);
            },
            None => {
                path.set_extension("zip");
            }
        }
        let response = self.sync_zip(name, path.to_str().unwrap(), &content);
        println!("zip dir elapsed time: {:?}", elapsed);
        return response;
    }

    fn _zip_file(&self, path: &str) {
        let path = std::path::Path::new(path);
        let name = path.into_iter().last().unwrap();
        let target_path = std::env::current_dir().unwrap().join(name);
        let target_file = std::fs::File::open(target_path.clone()).unwrap();
        let mut zip = zip::ZipWriter::new(target_file);
        let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Zstd);

        zip.start_file(name.to_string_lossy(), options).unwrap();
        let mut file = std::fs::File::open(path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        zip.write_all(&buffer[..]).unwrap();
        buffer.clear();
        zip.finish().unwrap();

        self.sync_file(target_path.to_str().unwrap());
    }

    fn zip_visit_dir(&self, entry_dir: &std::path::Path, dir: &std::path::Path, zip: &mut zip::ZipWriter<&mut std::io::Cursor<Vec<u8>>>, options: &zip::write::FileOptions) {
        let mut buffer = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        let path = entry.path();
                        let name = path.strip_prefix(entry_dir).unwrap().to_str().unwrap();

                        if file_type.is_dir() {
                            zip.add_directory(name, *options).unwrap();
                            self.zip_visit_dir(entry_dir, &path, zip, options);
                            println!("name: {:?}, path: {:?}", name, entry.path().to_str().unwrap());
                        }
                        else if file_type.is_file() {
                            zip.start_file(name, options.to_owned()).unwrap();
                            let mut file = std::fs::File::open(path).unwrap();
                            file.read_to_end(&mut buffer).unwrap();
                            zip.write_all(&buffer[..]).unwrap();
                            buffer.clear();
                        }
                    } 
                    else {

                    }
                }
            }
        }

    }

    pub fn sync_dir(&self, dir: &str, name: &str, client: &crate::network::client::NetworkClient, compress: bool) -> crate::compiler::compiler::SyncData {
        if compress {
            let response = self.zip_dir(dir, name, client);
            return response;
        }
        else {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                self.sync_dir(entry.path().to_str().unwrap(), name, client, compress);
                            }
                            else if file_type.is_file() {
                                self.sync_file(entry.path().to_str().unwrap());
                            }
                        } 
                        else {
        
                        }
                    }
                }
            }
            return crate::compiler::compiler::SyncData::default();
        }
    }

    pub fn sync_zip(&self, name: &str, filename: &str, filecontent: &std::borrow::Cow<[u8]>) -> crate::compiler::compiler::SyncData {
        let response = self.client.dist_zip_sync("dist/syncfile", name, filename, &filecontent);
        return response;
    }

    pub fn sync_file(&self, file: &str) {
        println!("sync file: {:?}", file);
        match self.client.dist_file_sync("dist/syncfile",file) {
            Ok(_response) => {
                println!("sync file respone");
            },
            Err(error) => {
                println!("sync file post failed. {:?}", error);
            },
        }
    }
    
    pub fn sync_toolchain(&self, path: &std::path::PathBuf) -> (std::ffi::OsString, std::ffi::OsString) {
        println!("path: {:?}", path);
        let mut path = std::path::PathBuf::from(path);
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\
        //msvc bin dir
        let mut response = self.sync_dir(path.to_str().unwrap(), "msvc", self.client, true);
        let toolchain_bin_path = response.toolchain_path.clone();

        //msvc include
        for _ in 0..3 {
            path.pop();
        }
        let include = path.join("include");

        if include.is_dir() {
            println!("sync dir: {:?}", path);
            response = self.sync_dir(include.to_str().unwrap(), "msvc", self.client, true);
        }
        let toolchain_include_path = response.toolchain_path.clone();
        return (toolchain_bin_path, toolchain_include_path);
    }

    pub fn sync_windows_kits(&self, path: &std::path::PathBuf) -> std::ffi::OsString {
        if path.is_dir() {
            let response = self.sync_dir(path.to_str().unwrap(), "kits", self.client, true);
            return response.windows_kits_path;
        }
        return std::ffi::OsString::new();
    }

    pub fn dist_compile(&self, msvc_compile_input: &crate::compiler::compiler::CompileInput) -> crate::compiler::compiler::CompileOutput {
        let response = self.client.dist_request_compile(msvc_compile_input);
        return response;
    }

    pub fn dist_kits_and_tool_pre_sync(&self, kits_path: &str, compiler_path: &str) -> (std::ffi::OsString, std::ffi::OsString, std::ffi::OsString) {
        let sync_info = crate::compiler::compiler::SyncData {
            sync_kind: std::ffi::OsString::from("kits, msvc"),
            toolchain_path: std::ffi::OsString::from(compiler_path),
            windows_kits_path: std::ffi::OsString::from(kits_path),
            file_path: std::ffi::OsString::new(),
            file_name: std::ffi::OsString::new(),
            digest: std::ffi::OsString::new(),
            is_exists: false,
        };
        let response = self.client.dist_kits_and_tool_pre_sync(&sync_info);

        let mut dist_msvc_include_path = std::ffi::OsString::new();
        let mut dist_msvc_compiler_path = response.toolchain_path;

        if dist_msvc_compiler_path.is_empty() {
            
        }
        else {
            let mut bin = std::path::PathBuf::from(dist_msvc_compiler_path.clone());
            bin.set_file_name("cl.exe");
            dist_msvc_compiler_path = bin.into_os_string();
            
            let mut inlcude = std::path::PathBuf::from(dist_msvc_compiler_path.clone());
            for _ in 0..3 {
                inlcude.pop();
            }
            dist_msvc_include_path = inlcude.join("include").into_os_string();
        }

        return (dist_msvc_compiler_path, dist_msvc_include_path, response.windows_kits_path)
    }

}



