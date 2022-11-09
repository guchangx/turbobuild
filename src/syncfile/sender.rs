use std::f32::consts::E;
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

    fn zip_dir(&self, dir: &str, client: &crate::network::client::NetworkClient) {
        let path = std::path::Path::new(dir);
        let name = path.into_iter().last().unwrap();
        let target_path = std::env::current_dir().unwrap().join(name);
        let target = std::fs::File::open(target_path.clone()).unwrap();
        let mut zip = zip::ZipWriter::new(target);
        let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Zstd);
        let start = std::time::Instant::now();
        self.zip_visit_dir(path, path, &mut zip, &options);
        zip.finish().unwrap();
        let elapsed = start.elapsed();
        self.sync_file(target_path.to_str().unwrap());
        println!("zip dir elapsed time: {:?}", elapsed);
    }

    fn zip_file(&self, path: &str) {
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

    fn zip_visit_dir(&self, entry_dir: &std::path::Path, dir: &std::path::Path, zip: &mut zip::ZipWriter<std::fs::File>, options: &zip::write::FileOptions) {
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

    pub fn sync_dir(&self, dir: &str, client: &crate::network::client::NetworkClient, compress: bool) {
        if compress {
            self.zip_dir(dir, client);
        }
        else {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                self.sync_dir(entry.path().to_str().unwrap(), client, compress);
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
        }
    }
    pub fn sync_file(&self, file: &str) {
        self.client.dist_file_sync("syncmsvc",file)
    }
    
    pub fn sync_tool_chain(&self, path: &str) {
        let mut path = std::path::PathBuf::from(path);

        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
        //msvc bin
        self.sync_file(path.to_str().unwrap());

        //msvc include 
        for _ in 0..4 {
            path.pop();
        }
        
        self.sync_dir(path.to_str().unwrap(), self.client, false);
    }

    pub fn dist_compile(&self, env: &crate::platform::windows::WindowsCompilerEnv, msvc_compile_input: &crate::compiler::compiler::CompileInput) {
        self.client.dist_request_compile(env, msvc_compile_input).unwrap();
    }

}



