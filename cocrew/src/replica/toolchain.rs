pub struct Property {
    replica_dir: String,
    original_toolchain_path: String,
    replica_toolchain_path: String,
}

impl Property {
    pub fn new(replica_dir: String, original_toolchain_path: String) -> Self {
        Property {
            replica_dir,
            original_toolchain_path,
            replica_toolchain_path: "".to_string(),
        }
    }
    
    pub fn fetch_replica_path(&self) -> String {
        let replica_path = common::util::get_working_path("Replica".to_string());
        println!("replica path: {:?}", replica_path);
        if let Some(path) = replica_path {
            let path = self.mapping_dir(path); 
            return path.display().to_string();
        }
        else {
            return "".to_string();
        }
    }

    fn mapping_dir(&self, replica_dir: String) -> std::path::PathBuf {
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\include
       let path = std::path::Path::new(&self.original_toolchain_path);
   
       let compiler_version = path.into_iter()
           .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe")
           .collect::<Vec<&std::ffi::OsStr>>()
           .first()
           .unwrap()
           .to_owned();
       
       let current_dir = std::path::PathBuf::from(replica_dir);
       let name = path.file_stem().unwrap();
       if name.to_str().unwrap().contains("86") || name.to_str().unwrap().contains("64") {
           let compiler_mapping_path = current_dir.join("MSVC").join(compiler_version).join(r"bin\Hostx64")
           .join(name);
           let _ = std::fs::create_dir_all(&compiler_mapping_path).expect("crate msvc compiler cl.exe dir failed.");
           return compiler_mapping_path;
       }
       else if name.to_string_lossy().contains("include") {
           let compiler_include_mapping_path = current_dir.join("MSVC").join(compiler_version).join("include");
           let _ = std::fs::create_dir_all(&compiler_include_mapping_path).expect("crate msvc compiler cl.exe dir failed.");
           return compiler_include_mapping_path;
       }
       else {
           return current_dir.join("unnamed");
       }
   }
}

