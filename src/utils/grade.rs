
extern crate sysinfo;

use sysinfo::{CpuExt, SystemExt};
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct LocalGrade {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub device_name: String,
}

impl LocalGrade {

    pub fn fetch_grade(&self) -> Self {
        return self.to_owned();
    }

    pub fn init_grade() -> Self {
        let grade = LocalGrade {cpu_usage: 0.0, memory_usage: 0.0, device_name: "".to_string()};
        return grade;
    }
} 

pub fn calculate_machine_residual_performance(grade: std::sync::Arc<std::sync::Mutex<LocalGrade>>) {

    let _handle = std::thread::spawn( move || {
 
        let mut sys = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything())
        );
        let mut grade = grade.lock().unwrap();
        grade.device_name = sys.host_name().unwrap();

        loop {
            sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
            sys.refresh_memory();
            grade.cpu_usage = sys.global_cpu_info().cpu_usage();
            let memory_usage = ((sys.used_memory() as f32 * 100 as f32 / sys.total_memory() as f32) * 1000.0).round() / 1000.0;
            grade.memory_usage = memory_usage;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

