

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

impl Arch {
    pub fn format(arch: &str) -> Arch {
        if arch == "x86" {
            return Arch::x86;
        }
        else if arch == "x64" {
            return Arch::x64;
        }
        else if arch == "arm" {
            return Arch::arm;
        }
        else if arch == "arm64" {
            return Arch::arm64;
        }
        else if arch == "Hostx64" {
            return Arch::x64;
        }
        else if arch == "Hostx86" {
            return Arch::x86;
        }
        else {
            return Arch::unknown;
        }
    }   
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
            replica_dir: Self::access_or_create_replica_dir(),
            original_toolchain_path: String::new(),
            replica_toolchain_versions: Self::load_replica_toolchain(),
        }
    }

    pub fn new(original_toolchain_path: &str) -> Self {
        let path = Self::access_or_create_replica_dir();
        Property {
            replica_dir: path,
            original_toolchain_path: original_toolchain_path.to_string(),
            replica_toolchain_versions: Vec::new(),
        }
    }

    fn access_or_create_replica_dir() -> String {
        let path = tools::utils::access_or_create_working_path("Replica");
        return path;
    }

    pub fn access_replica_toolchain_path(&self) -> String {
        let mut path = std::path::PathBuf::from(&self.replica_dir);

        if self.original_toolchain_path.contains(r"\MSVC\") {
            path.push(r"MSVC");
            log::debug!("original toolchain path: {}", self.original_toolchain_path);
            if let Some(version) = Self::parse_version_from_path(&self.original_toolchain_path) {
                path.push(&version);

                if self.original_toolchain_path.contains(&format!(r"{}\bin", version)) {
                    path.push("bin");
                }

                if self.original_toolchain_path.contains(&format!(r"{}\include", version)) {
                    path.push("include");
                }

                if self.original_toolchain_path.contains(&format!(r"{}\atlmfc", version)) {
                    path.push("atlmfc");
                    path.push("include");
                }
            }
        }
        else if self.original_toolchain_path.contains(r"\Windows Kits\") {
            if let Some(start) = self.original_toolchain_path.find(r"Windows Kits\") {
                let sub = &self.original_toolchain_path[start ..];
                path.push(sub);
            }
        }

        if path.exists() && path.is_dir() {
            if path.exists() {
            
            }
            else {
                std::fs::create_dir_all(&path).unwrap();
            }
        }
        return path.to_string_lossy().to_string();
    }

    pub fn parse_version_from_path(path: &str) -> Option<String> {
        let path = std::path::PathBuf::from(path);

        let mut iter = path.components().skip_while(|&item| {
                let item = item.as_os_str().to_string_lossy();
                let vec = item.split('.').collect::<Vec<&str>>();
                if vec.len() >= 3 {
                    return false;
                }
                else {
                    return true;                
                }
        });
        
        if let Some(version) = iter.next() {
            let version = version.as_os_str().to_string_lossy().to_string();
            return Some(version);
        }
        else {
            return None;
        }
    }

    pub fn load_replica_toolchain() -> Vec<CompilerVersion> {
        let mut versions = Vec::new();

        if let Some(path) = tools::utils::access_working_path("Replica") {
            let replica = std::path::PathBuf::from(path.clone()).join("MSVC");
        
            if replica.exists() {
                for version_entry in replica.read_dir().unwrap() {
                    if let Ok(version_entry) = version_entry {

                         for host_entry in version_entry.path().join("bin").read_dir().unwrap() {
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
                log::warn!("can't find msvc in replica dir. {}", path);
            }
        }
        else {
            log::warn!("can't find replice in target dir so return emprty cversion.");
        }
        return versions;
    } 

    pub async fn check_resource_and_judge_sync(resources: Vec<crate::replica::toolchain::CrewsResource>) {
    
        let compiler_env = crate::platform::windows::WindowsCompilerEnv::default();
        let bin_dir = compiler_env.compiler_path.clone();
        let version = compiler_env.msvc_version;

        let devicename = crate::fingerprint::gather::SystemInfo::fetch_devicename();
        
        for item in resources {
            // skip when addr is localhost when not in local machine.

            if (item.addr == "localhost" || item.addr =="127.0.0.1") && item.devicename != devicename {
                continue;
            }

            let mut exist = false;
            for compiler in item.compiler_versions {
                if compiler.version == version {
                    exist = true;
                }
            }

            if !exist {
                log::warn!("check resource {} not has msvc {}, so sync it. path: {:?}", item.addr, version, bin_dir.clone());
                let addr =  item.addr.clone();

                let version_= version.clone();
                if let Some(name) = bin_dir.clone().file_name() {
                    if name.to_str() == Some("bin") {
                        let path = bin_dir.clone();
                        tokio::spawn(async move {
                            Self::sync_compiler_toolchain(path.to_str().unwrap(), &item.addr).await;
                            log::info!("sync compiler toolchain {} to {} finished.", version_, item.addr);

                            let version_x64 = CompilerVersion {
                                version: version_.clone(),
                                host: Arch::x64,
                                target: Arch::x64,
                            };
                            
                            let version_x84 = CompilerVersion {
                                version: version_,
                                host: Arch::x64,
                                target: Arch::x86,
                            };

                            let resource = CrewsResource {
                                username: item.username,
                                aliasname: item.aliasname, 
                                devicename: item.devicename,
                                addr: item.addr,
                                compiler_versions: [version_x64, version_x84].to_vec(),
                            };

                            let info = serde_json::to_string(&resource).unwrap();
                            log::info!("report synced crew resource: {:?}", info);
                            crate::communicate::notifier::NotificationSender::notify_once(info).await;
                        });
                    }

                    let winkits_includes = compiler_env.winkits_includes_path.clone();
                    let msvc_includes = compiler_env.msvc_includes_path.clone();
                    tokio::spawn(async move {
                        for dir in winkits_includes {
                            Self::sync_compiler_includes(&dir.to_str().unwrap(), &addr).await;
                        }

                        for dir in msvc_includes {
                            Self::sync_compiler_includes(&dir.to_str().unwrap(), &addr).await;
                        }
                        
                    });
                }
            }
            else {
                log::info!("check resource addr: {} has msvc {}, not need to sync.", item.addr, version);
            }
        }
    }
    
    pub async fn sync_compiler_toolchain(path: &str, addr: &str) {
        let packager = crate::communicate::packager::Packager::default();
        packager.toolchain(path, addr).await;
    }
    
    pub async fn sync_compiler_includes(path: &str, addr: &str) {
        let packager = crate::communicate::packager::Packager::default();
        packager.includes(path, addr).await;
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

        let path = Property::access_or_create_replica_dir();
        println!("path: {:?}", path);
    }

    #[test]
    fn test() {
        let path = r" C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.44.35207\atlmfc\include";
        let sub = format!(r"{}\atlmfc\", "14.44.35207");
        let contain = path.contains(&sub);
        println!("contain: {}", contain);
    }
}