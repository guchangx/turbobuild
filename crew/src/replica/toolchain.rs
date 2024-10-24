use tonic::Streaming;


pub struct Property {
    replica_dir: String,
    original_toolchain_path: String,
    replica_toolchain_versions: Vec<CompilerVersion>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompilerVersion {
    pub version: String,
    pub host: Arch,
    pub target: Arch,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum Arch {
    unknown = 0,
    arm = 1,
    arm64 = 2,
    x86 = 3,
    x64 = 4,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrewsResource {
    pub username: String,
    pub aliasname: String, 
    pub devicename: String,
    pub addr: String,
    pub compiler_versions: Vec<CompilerVersion>,
}

impl Property {
    
    pub fn default() -> Self {
        return Property {
            replica_dir: Self::load_or_create_replica_dir(),
            original_toolchain_path: String::new(),
            replica_toolchain_versions: Self::load_replica_toolchain(),
        }
    }

    pub fn new(replica_dir: String, original_toolchain_path: String) -> Self {
        Property {
            replica_dir,
            original_toolchain_path,
            replica_toolchain_versions: Vec::new(),
        }
    }

    fn load_or_create_replica_dir() -> String {
        if let Some(path) = common::utils::get_working_path("Replica".to_string()) {
            return path;
        }
        else {
            let mut dir = std::env::current_exe().unwrap();
            log::info!("current dir: {:?}", dir);
            
            let mut path = "";
            if dir.components().any(|item| item.as_os_str() == "deps") {
                dir.pop();
                dir.pop();
                dir.push("Replica");
                std::fs::create_dir(&dir).unwrap();
                path = dir.to_str().unwrap();
            }
            else {
                dir.pop();
                dir.push("Replica");
                std::fs::create_dir(&dir).unwrap();
                path = dir.to_str().unwrap();
            }

            return path.to_string();
        }
    }

    pub fn fetch_replica_path(&self) -> String {
        if let Some(path) = common::utils::get_working_path("Replica".to_string()) {
            return path;
        }
        else {
            let mut dir = std::env::current_exe().unwrap();
            log::info!("current dir {:?}", dir);
            
            let mut path = "";
            if dir.components().any(|item| item.as_os_str() == "deps") {
                dir.pop();
                dir.pop();
                dir.push("Replica");
                if !dir.exists() {
                    std::fs::create_dir(&dir).unwrap();
                }
                path = dir.to_str().unwrap();
            }
            else {
                dir.pop();
                dir.push("Replica");
                
                if !dir.exists() {
                    std::fs::create_dir(&dir).unwrap();
                }
                path = dir.to_str().unwrap();
            }

            return path.to_string();
        }
    }

    pub fn load_replica_toolchain() -> Vec<CompilerVersion> {
        let mut versions = Vec::new();

        if let Some(path) = common::utils::get_working_path("Replica".to_string()) {
            let mut replica = std::path::PathBuf::from(path);
            replica.push("MSVC");
        
            if replica.exists() {
                for version_entry in replica.read_dir().unwrap() {
                    if let Ok(version_entry) = version_entry {
                         for host_entry in version_entry.path().read_dir().unwrap() {
                            if let Ok(host_entry) = host_entry {
                                for target_entry in host_entry.path().read_dir().unwrap() {
                                    if let Ok(target_entry) = target_entry {   

                                        let mut host = Arch::unknown;
                                        if host_entry.file_name() == "Hostx64" {
                                            host = Arch::x64;
                                        }
                                        else if host_entry.file_name() == "Hostx86"{
                                            host = Arch::x86
                                        }
                                        else if host_entry.file_name() == "arm" {
                                            host = Arch::arm
                                        }
                                        else if host_entry.file_name() == "arm64" {
                                            host = Arch::arm64
                                        }

                                        let mut target = Arch::unknown;
                                        if target_entry.file_name() == "x64" {
                                            target = Arch::x64;
                                        }
                                        else if target_entry.file_name() == "x86" {
                                            target = Arch::x86;
                                        }
                                        else if target_entry.file_name() == "arm" {
                                            target = Arch::arm;
                                        }
                                        else if target_entry.file_name() == "arm64" {
                                            target = Arch::arm64;                
                                        }

                                        if host != Arch::unknown || target != Arch::unknown {
                                            let ver = CompilerVersion {
                                                version: version_entry.file_name().to_string_lossy().to_string(),
                                                host: host,
                                                target: target,
                                            };
                                            versions.push(ver);
                                        }
                                    }
                                }
                            }
                         }
                    }
                }
                return versions;
            }
            else {
                println!("can't find msvc in replica dir.");
            }
        }
        else {
            println!("can't find replice in target dir.");
        }
        return versions;
    } 

    pub async fn check_resource_and_judge_sync(resources: Vec<crate::replica::toolchain::CrewsResource>) {
    
        let compiler_env = crate::platform::windows::WindowsCompilerEnv::default();
        let bin_dir = compiler_env.compiler_path;

        for item in resources {
            let mut has = false;
            for compiler in item.compiler_versions {
                if compiler.version == compiler_env.msvc_version {
                    has = true;
                }
            }
            if !has {
                println!("{} not has msvc {}, so sync it. path: {:?}", item.addr, compiler_env.msvc_version, bin_dir.clone());
            
                if let Some(name) = bin_dir.clone().file_name() {
                    if name.to_str() == Some("bin") {
                        let path = bin_dir.clone();
                        tokio::spawn(async move {
                            Self::sync_compiler_toolchain(path.to_str().unwrap(), &item.addr).await;
                        });
                    }
                } 
            }
        }
    }
    
    pub async fn sync_compiler_toolchain(path: &str, addr: &str) {
        let packager = crate::communicate::packager::Packager::default();
        packager.toolchain(path, addr).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //cargo test --package cocrew --tests load_replica -- --show-output
    fn load_replica() {
        println!("test load replica toolchain");
        let versions = Property::load_replica_toolchain();
        println!("path version: {:?}", versions);

        let path = Property::load_or_create_replica_dir();
        println!("path: {:?}", path);
    }
}