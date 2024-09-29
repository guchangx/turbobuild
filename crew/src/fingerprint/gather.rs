
extern crate sysinfo;
use sysinfo::System;

pub struct SystemInfo {
    pub username: String,
    pub aliasname: String,
    pub role: u32,
    pub status: u32,
    pub action: u32,
    pub addr: String,
    pub physical_cores: u32,
    pub virtual_cores: u32,
    pub cpu_usage: f32,
    pub cpu_frequency: f32,
    
    pub memory: f32,
    pub memory_usage: f32,
    
    pub os_version: String,

    pub description: String,
}

impl SystemInfo {
    pub fn new() -> Self {

        let username = SystemInfo::fetch_username();
        let devicename = SystemInfo::fetch_devicename();
        println!("devicename{}", devicename);
        let aliasname = SystemInfo::fetch_aliasname();
        let role = 0;
        let addr = SystemInfo::fetch_addr();

        let (os, physical_cores, virtual_cores, cpu_usage) = SystemInfo::fetch_cpu_info();
        let (memory_total, memory_usage) = SystemInfo::fetch_memory_info();
        
        return SystemInfo {
            username,
            aliasname,
            role,
            status: 0,
            action: 0,
            addr,
            physical_cores,
            virtual_cores,
            cpu_usage,
            cpu_frequency: 0.0,

            memory: memory_total,
            memory_usage,
            os_version: os,

            description: "".to_string(),
        }
    }
    fn fetch_username() -> String {
        let username = std::env::var("USERNAME").unwrap_or_else(|_| std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()));
        return username;
    }
    
    fn fetch_devicename() -> String {
        let device_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
        return device_name;
    }
    
    fn fetch_aliasname() -> String {
        return "".to_string();
    }
    
    fn fetch_addr() -> String {
        let mut networks = sysinfo::Networks::new_with_refreshed_list();
        for (interface_name, network) in &networks {
            println!("Ip Networks: {:?}", network.ip_networks());
        }
        return "".to_string();
    }

    fn fetch_cpu_info() -> (String, u32, u32, f32) {
        
        let mut sys = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything())
        );
        
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        
        sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        
        let usage = sys.global_cpu_usage();
        println!("cpu usage: {}", usage);
        
        let mut physical_cores: u32 = 0;
        if let Some(cores) = sys.physical_core_count() {
            physical_cores = cores as u32;
            println!("physical cores {:?}", physical_cores);    
        };
        
        
        let system_name = sysinfo::System::name();
        println!("system name {:?}", system_name);
        let mut os_version = "unknown".to_string();
        if let Some(os) = sysinfo::System::long_os_version() {
            os_version = os;
            println!("Long OS Version: {:?}", os_version);
        }
        
        println!("CPU Architecture: {:?}", sysinfo::System::cpu_arch());
        
        
        let virtual_cores = sys.cpus().len() as u32;
        println!("cpu cores {}", virtual_cores);
        let mut frequency: Vec<u64> = vec![];
        for cpu in sys.cpus() {
            frequency.push(cpu.frequency());
            println!("{}", cpu.frequency());
        }
        
        return (os_version, physical_cores, virtual_cores, usage);
    }

    fn fetch_memory_info() -> (f32, f32) {
        
        let mut sys = System::new();
        sys.refresh_memory();
        
        let used = sys.used_memory() as f32;
        let total = sys.total_memory() as f32;
    
        let memory_usage = ((used * 100 as f32 / total) * 1000.0).round() / 1000.0;
        println!("memory usage: {}", memory_usage);
        return (total, memory_usage);
    }
 }

