
use sysinfo::System;

#[derive(Debug, serde::Serialize, serde::Deserialize)]

pub struct SystemInfo {
    pub username: String,
    pub devicename: String,
    pub aliasname: String,
    pub role: u32,
    pub status: u32,
    pub action: u32,
    pub addr: String,
    pub physical_cores: u32,
    pub virtual_cores: u32,
    pub cpu_usage: f32,
    pub cpu_frequency: Vec<u64>,
    pub memory_total: f32,
    pub memory_usage: f32,
    pub os_version: String,
    pub description: String,
}

impl SystemInfo {
    pub fn new() -> Self {

        let username = SystemInfo::fetch_username();
        let devicename = SystemInfo::fetch_devicename();
        let aliasname = SystemInfo::fetch_aliasname();
        let role = 0;
        //let addr = SystemInfo::fetch_addr();
        let addr = "".to_string();
        let os = SystemInfo::fetch_os_version();
        let (physical_cores, virtual_cores, frequency) = SystemInfo::fetch_cpu_info();
        
        let (cpu_used, _memory_used) = SystemInfo::fetch_cpu_and_memory_usage();
        
        let (memory_total, memory_usage) = SystemInfo::fetch_memory_info();
        
        return SystemInfo {
            username,
            devicename,
            aliasname,
            role,
            status: 0,
            action: 0,
            addr,
            physical_cores,
            virtual_cores,
            cpu_usage: cpu_used,
            cpu_frequency: frequency,
            memory_total: memory_total,
            memory_usage,
            os_version: os,
            description: "".to_string(),
        }
    }
    pub fn fetch_username() -> String {
        let username = std::env::var("USERNAME").unwrap_or_else(|_| std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()));
        return username;
    }
    
    pub fn fetch_devicename() -> String {
        let device_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
        return device_name;
    }
    
    pub fn fetch_aliasname() -> String {
        return "".to_string();
    }
    
    fn fetch_addr() -> String {
        let networks = sysinfo::Networks::new_with_refreshed_list();
        for (name, network) in &networks {
            for ip in network.ip_networks() {
                if ip.addr.is_ipv4() {
                    println!("addr: {} ipv4:{}", name, ip.addr.to_string());
                }
            }
        }
        return "".to_string();
    }

    fn fetch_cpu_info() -> (u32, u32, Vec<u64>) {
        
        let mut sys = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything())
        );
        
        sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        
        let mut physical_cores: u32 = 0;
        if let Some(cores) = sys.physical_core_count() {
            physical_cores = cores as u32;
        };
        
        let virtual_cores = sys.cpus().len() as u32;
        
        let mut frequency: Vec<u64> = vec![];
        
        for cpu in sys.cpus() {
            let f = cpu.frequency();
            if !frequency.contains(&f) {
                frequency.push(f);                
            }
        }
        return (physical_cores, virtual_cores, frequency);
    }
    
    pub fn fetch_cpu_and_memory_usage() -> (f32, f32) {
        
        let mut sys = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything())
        );
        
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        
        sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        
        let cpu_usage = sys.global_cpu_usage();
        let mut sys = System::new();
        sys.refresh_memory();
        
        let memory_used = sys.used_memory() as f32;
        
        return (cpu_usage, memory_used);
    }
    
    fn fetch_os_version() -> String {
        let mut os_version = "unknown".to_string();
        if let Some(os) = sysinfo::System::long_os_version() {
            os_version = os;
        }
        return os_version;
    }

    fn fetch_memory_info() -> (f32, f32) {
        
        let mut sys = System::new();
        sys.refresh_memory();
        
        let used = sys.used_memory() as f32;
        let total = sys.total_memory() as f32;
        let total_megabytes = (total / 1024.0 / 1024.0 / 1024.0 * 10.0).round() / 10.0;
        let memory_usage = ((used * 100 as f32 / total) * 1000.0).round() / 1000.0;
        return (total_megabytes, memory_usage);
    }

}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegisterInfo {
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub passcode: String,
    pub core: u16,
    pub memory: f32,
}

impl RegisterInfo {
    pub fn new() -> Self {
        let username = SystemInfo::fetch_username();
        let devicename = SystemInfo::fetch_devicename();
        let (_, core, _) = SystemInfo::fetch_cpu_info();
        let (memory, _) = SystemInfo::fetch_memory_info();
        return Self {
            username,
            devicename,
            addr: "".to_string(),
            core: core as u16,
            memory,
            passcode: "".to_string()
        };
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Resource {
    pub username: String, 
    pub aliasname: String,
    pub addr: String,
    pub winkits_includes_path: Vec<std::ffi::OsString>,
    pub compiler_path: std::ffi::OsString,
    pub msvc_includes_path: std::ffi::OsString,
    pub msvc_version: String,
}

impl Resource {
    pub fn new() -> Self {
        #![allow(unused)]
        let username = SystemInfo::fetch_username();
        let aliasname = SystemInfo::fetch_aliasname();
        let addr = "".to_string();
        let compiler = "".to_string();
        let msvc = vec![""];
        let version = vec![""];
        
        return Self {
            username: "".to_string(),
            aliasname: "".to_string(),
            addr: "".to_string(),
            winkits_includes_path: vec![],
            compiler_path: std::ffi::OsString::new(),
            msvc_includes_path: std::ffi::OsString::new(),
            msvc_version: "".to_string(),
        }
    }

    fn fetch_compiler_path() -> std::ffi::OsString {
        
        let replice = tools::utils::access_working_path("Replice");
        return std::ffi::OsString::new(); 
    }
    
    fn fetch_msvc_includes_path() -> std::ffi::OsString {
        return std::ffi::OsString::new();
    }

    fn fetch_winkits_includes_path() -> Vec<std::ffi::OsString> {
        return vec![];
    }

    fn fetch_msvc_version() -> String {
        return "".to_string();
    }
}

