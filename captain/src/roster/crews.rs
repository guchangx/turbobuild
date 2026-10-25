
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct CrewConstitution {
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct CrewRegister {
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub passcode: String,
    pub core: u32,
    pub frequency: Vec<u64>,
    pub memory: f32,
}

fn default()-> bool {
    true
}

#[derive(serde::Deserialize, Clone, serde::Serialize, Debug, PartialEq)]
pub struct Task {
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub core: u32,
    pub frequency: Vec<u64>,
    pub memory: f32,
    pub running: u32,
    pub max: u32,
    #[serde(default = "default")]
    pub online: bool,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            username: String::new(),
            devicename: String::new(),
            addr: String::new(),
            core: 0,
            frequency: Vec::new(),
            memory: 0.0,
            running: 0,
            max: 0,
            online: true,
        }
    }
}


pub struct TaskManager {
    tasks: Vec<Task>,
}

impl TaskManager {
    pub fn new() -> Self {
        return Self {
            tasks: Vec::new()
        }
    }

    pub fn add(&mut self, task: Task) {
        if let Some(existing) = self.tasks.iter_mut().find(|t| t.username == task.username && t.devicename == task.devicename) {
            existing.core = task.core;
            existing.memory = task.memory;
            existing.running = task.running;
            existing.max = task.max;
            existing.addr = task.addr.clone();
            existing.frequency = task.frequency.clone();
            existing.online = task.online;
        } else {
            self.tasks.push(task.clone());
        }
    }

    pub fn add_from_crew(&mut self, crew: &CrewRegister) {

        let task = Task {
            username: crew.username.clone(),
            devicename: crew.devicename.clone(),
            addr: crew.addr.clone(),
            core: crew.core,
            frequency: crew.frequency.clone(),
            memory: crew.memory,
            running: 0,
            max: max(crew.core, crew.frequency.first().copied().filter(|f| *f > 0).unwrap_or(3000) as f64),
            online: true,
        };
        self.add(task);
    }

    pub fn alls(&self) -> &Vec<Task> {
        &self.tasks
    }

    pub fn check(&self, addr: &str, username: &str, devicename: &str) -> Vec<Task> {
        if addr.is_empty() && username.is_empty() && devicename.is_empty() {
            return self.tasks.clone();
        }
        else {
            if let Some(task) = self.tasks.iter().find(|item| item.addr == addr && item.username == username && item.devicename == devicename) {
                return vec![task.clone()];
            }
            else {
                return Vec::new();
            }
        }
    }

    pub fn remove(&mut self, addr: &str, username: &str, devicename: &str) -> Option<Task> {
        let index = self.tasks.iter().position(|arg| {
            if (username.is_empty() || arg.username == username)
                && (devicename.is_empty() || arg.devicename == devicename)
                && (addr.is_empty() || arg.addr == addr) {
                return arg.online;
            }
            else {
                return false;
            }
        });

        match index {
            Some(index) => {
                let task = &mut self.tasks[index];
                task.online = false;
                task.running = 0;
                Some(task.clone())
            },
            None => None
        }
    }
}

fn weight(frequency: f64) -> f64 {
    frequency / 3000.0
}

pub fn max(core: u32, frequency: f64) -> u32 {
    ((core as f64 * 1.5 * weight(frequency)) as u32).max(1) // max running tasks: core * 1.5 * (frequency / 3000), at least 1
}

pub struct ConstitutionList {
    crews: Vec<CrewConstitution>,
    #[allow(dead_code)]
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
}

impl ConstitutionList {
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        return ConstitutionList {
            crews: Vec::new(),
            common,
        };
    }
    
    pub fn check(&mut self, addr: &str, username: &str, devicename: &str) -> Vec<CrewConstitution> {
        if addr.is_empty() && username.is_empty() && devicename.is_empty() {
            let crews = self.crews.clone();
            return crews;
        }
        else {
            let mut constitution = Vec::new();
            for crew in self.crews.clone() {
                if (!crew.addr.is_empty() && crew.addr == addr) 
                    && (crew.devicename == devicename)
                    && (!crew.username.is_empty() && crew.username == username) {
                        constitution.push(crew);
                }
            }
            return constitution;
        }
    }
    
    pub fn add(&mut self, crew: CrewConstitution) {
        self.crews.push(crew);
    }
    
    pub fn remove(&mut self, crew: CrewRegister) -> bool {
        let index = self.crews.iter().position(|arg| arg.username == crew.username && arg.addr == crew.addr);
        match index {
            Some(index) => {
                self.crews.remove(index);
                true
            },
            None => false
        }
    }
    
    pub fn keepalive(&self, _crew: CrewConstitution) {
        
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum Arch {
    unknown = 0,
    arm = 1,
    arm64 = 2,
    x86 = 3,
    x64 = 4,
}

impl From<i32> for Arch {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::unknown,
            1 => Self::arm,
            2 => Self::arm64,
            3 => Self::x86,
            4 => Self::x64,
            _ => panic!("Unknown value"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct CompilerVersion {
    pub version: String,
    pub host: Arch,
    pub target: Arch,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CrewResource {
    pub username: String, 
    pub aliasname: String,
    pub devicename: String,
    pub addr: String,
    pub compiler_versions: Vec<CompilerVersion>,
}

#[derive(Debug, Clone)]
pub struct ResourceList {
    crews: Vec<CrewResource>,
    #[allow(dead_code)]
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
}

impl ResourceList {
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        return ResourceList {
            crews: Vec::new(),
            common,
        };
    }
    pub fn add(&mut self, crew: CrewResource) {
        self.crews.push(crew);
    }

    pub fn update(&mut self, crew: CrewResource) {
        if !self.crews.iter().any(|item| item.addr == crew.addr && item.devicename == crew.devicename && item.username == crew.username) {
            self.add(crew);
        }
        else {
            self.crews.iter_mut().for_each(|item| {
                if item.addr == crew.addr && item.devicename == crew.devicename && item.username == crew.username {
                    item.aliasname = crew.aliasname.clone();
                    item.compiler_versions = crew.compiler_versions.clone();
                }
            });
        }
    }

    pub fn remove(&mut self, addr: &str, username: &str, devicename: &str) -> bool {
        let index = self.crews.iter().position(|arg| { 
            if (username.is_empty() || arg.username == username) && (devicename.is_empty() || arg.devicename == devicename) && arg.addr == addr {
                return true;
            }
            else {
                return false;
            }
        });

        match index {
            Some(index) => {
                self.crews.remove(index);
                true
            },
            None => false
        }
    }

    pub fn check(&mut self, addr: &str, username: &str, devicename: &str) -> Vec<CrewResource> {
        if addr.is_empty() && username.is_empty() && devicename.is_empty() {
            let crews = self.crews.clone();
            return crews;
        }
        else {
            let mut resource = Vec::new();
            for crew in self.crews.clone() {
                if (!crew.addr.is_empty() && crew.addr == addr) 
                    && (crew.devicename == devicename)
                    && (!crew.username.is_empty() && crew.username == username) {
                        resource.push(crew);
                }
            }
            return resource
        }
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_manager_new_and_check_empty() {
        let tm = TaskManager::new();
        assert!(tm.check("", "", "").is_empty());
    }

    #[test]
    fn test_task_manager_add_and_check_all() {
        let mut tm = TaskManager::new();
        let task1 = Task {
            username: "alice".to_string(),
            devicename: "host-a".to_string(),
            addr: "192.168.1.10".to_string(),
            core: 8,
            frequency: vec![2400, 2400],
            memory: 16.0,
            running: 0,
            max: 64,
            online: true,
        };
        let task2 = Task {
            username: "bob".to_string(),
            devicename: "host-b".to_string(),
            addr: "192.168.1.20".to_string(),
            core: 4,
            frequency: vec![3200],
            memory: 8.0,
            running: 1,
            max: 16,
            online: true,
        };

        tm.add(task1.clone());
        tm.add(task2.clone());

        let all = tm.check("", "", "");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], task1);
        assert_eq!(all[1], task2);
    }

    #[test]
    fn test_task_manager_add_duplicate_updates_existing() {
        let mut tm = TaskManager::new();
        let initial = Task {
            username: "alice".to_string(),
            devicename: "host-a".to_string(),
            addr: "192.168.1.10".to_string(),
            core: 8,
            frequency: vec![2400],
            memory: 16.0,
            running: 0,
            max: 64,
            online: true,
        };
        tm.add(initial);

        // Update task with same username and devicename, but changed core/memory/running/max/addr
        let updated = Task {
            username: "alice".to_string(),
            devicename: "host-a".to_string(),
            addr: "192.168.1.11".to_string(),
            core: 16,
            frequency: vec![2400], // note: frequency is not updated by add
            memory: 32.0,
            running: 3,
            max: 128,
            online: true,
        };
        tm.add(updated);

        let all = tm.check("", "", "");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].core, 16);
        assert_eq!(all[0].memory, 32.0);
        assert_eq!(all[0].running, 3);
        assert_eq!(all[0].max, 128);
        assert_eq!(all[0].addr, "192.168.1.11");
    }

    #[test]
    fn test_task_manager_add_from_crew() {
        let mut tm = TaskManager::new();
        let crew = CrewRegister {
            username: "charlie".to_string(),
            devicename: "workstation".to_string(),
            addr: "10.0.0.5".to_string(),
            passcode: "secret123".to_string(),
            core: 6,
            frequency: vec![3000, 3000],
            memory: 32.0,
        };

        tm.add_from_crew(&crew);

        let tasks = tm.check("", "", "");
        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        assert_eq!(task.username, "charlie");
        assert_eq!(task.devicename, "workstation");
        assert_eq!(task.addr, "10.0.0.5");
        assert_eq!(task.core, 6);
        assert_eq!(task.frequency, vec![3000, 3000]);
        assert_eq!(task.memory, 32.0);
        assert_eq!(task.running, 0);
        assert_eq!(task.max, 9); // core * 1.5 * (3000 / 3000) = 6 * 1.5 = 9
    }

    #[test]
    fn test_max_formula() {
        assert_eq!(max(8, 3000.0), 12); // 8 * 1.5 * 1.0
        assert_eq!(max(8, 2400.0), 9);  // 8 * 1.5 * 0.8 = 9.6 -> 9
        assert_eq!(max(8, 6000.0), 24); // 8 * 1.5 * 2.0
        assert_eq!(max(0, 3000.0), 1);  // at least 1

        // frequency missing or 0 falls back to 3000 MHz
        let mut tm = TaskManager::new();
        let mut crew = CrewRegister {
            username: "u".to_string(),
            devicename: "d".to_string(),
            addr: "10.0.0.1".to_string(),
            passcode: "".to_string(),
            core: 8,
            frequency: vec![],
            memory: 16.0,
        };
        tm.add_from_crew(&crew);
        assert_eq!(tm.alls()[0].max, 12);
        crew.frequency = vec![0];
        tm.add_from_crew(&crew);
        assert_eq!(tm.alls()[0].max, 12);
        crew.frequency = vec![2400, 3200];
        tm.add_from_crew(&crew);
        assert_eq!(tm.alls()[0].max, 9); // first frequency is used
    }

    #[test]
    fn test_task_manager_check_specific() {
        let mut tm = TaskManager::new();
        let task = Task {
            username: "david".to_string(),
            devicename: "pc-1".to_string(),
            addr: "192.168.1.50".to_string(),
            core: 4,
            frequency: vec![2500],
            memory: 8.0,
            running: 0,
            max: 16,
            online: true,
        };
        tm.add(task.clone());

        // Exact match
        let found = tm.check("192.168.1.50", "david", "pc-1");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], task);

        // Mismatched addr
        let not_found = tm.check("192.168.1.99", "david", "pc-1");
        assert!(not_found.is_empty());

        // Mismatched username
        let not_found_user = tm.check("192.168.1.50", "eve", "pc-1");
        assert!(not_found_user.is_empty());

        // Mismatched devicename
        let not_found_device = tm.check("192.168.1.50", "david", "pc-2");
        assert!(not_found_device.is_empty());
    }

    #[test]
    fn test_task_manager_remove_exact_and_wildcard() {
        let mut tm = TaskManager::new();
        let task1 = Task {
            username: "user1".to_string(),
            devicename: "dev1".to_string(),
            addr: "192.168.1.1".to_string(),
            core: 4,
            frequency: vec![],
            memory: 8.0,
            running: 0,
            max: 16,
            online: true,
        };
        let task2 = Task {
            username: "user2".to_string(),
            devicename: "dev2".to_string(),
            addr: "192.168.1.2".to_string(),
            core: 4,
            frequency: vec![],
            memory: 8.0,
            running: 0,
            max: 16,
            online: true,
        };
        tm.add(task1);
        tm.add(task2);

        // Exact removal
        assert!(tm.remove("192.168.1.1", "user1", "dev1").is_some());
        assert_eq!(tm.check("", "", "").iter().filter(|t| t.online).count(), 1);

        // Removing already-removed task returns None
        assert!(tm.remove("192.168.1.1", "user1", "dev1").is_none());

        // Wildcard removal (empty username and devicename matches any on that addr)
        assert!(tm.remove("192.168.1.2", "", "").is_some());
        assert_eq!(tm.check("", "", "").iter().filter(|t| t.online).count(), 0);

        // Remove from empty returns None
        assert!(tm.remove("192.168.1.2", "", "").is_none());
    }

    #[test]
    fn test_constitution_list_add_check_remove() {
        let mut cl = ConstitutionList::new(std::sync::Weak::new());
        assert!(cl.check("", "", "").is_empty());

        let const1 = CrewConstitution {
            username: "user1".to_string(),
            devicename: "dev1".to_string(),
            aliasname: "alias1".to_string(),
            role: 1,
            status: 0,
            action: 0,
            addr: "192.168.0.100".to_string(),
            physical_cores: 8,
            virtual_cores: 16,
            cpu_usage: 12.5,
            cpu_frequency: vec![3200],
            memory_total: 32.0,
            memory_usage: 4.5,
            os_version: "Windows 11".to_string(),
            description: "Test node".to_string(),
        };

        cl.add(const1.clone());
        let all = cl.check("", "", "");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].username, "user1");

        // Specific check matching all 3
        let matched = cl.check("192.168.0.100", "user1", "dev1");
        assert_eq!(matched.len(), 1);

        // Specific check mismatch
        let unmatched = cl.check("192.168.0.100", "user1", "dev2");
        assert!(unmatched.is_empty());

        // Keepalive placeholder does not panic
        cl.keepalive(const1.clone());

        // Remove using CrewRegister (matches username and addr)
        let reg = CrewRegister {
            username: "user1".to_string(),
            devicename: "dev1".to_string(),
            addr: "192.168.0.100".to_string(),
            passcode: "".to_string(),
            core: 8,
            frequency: vec![],
            memory: 32.0,
        };
        assert!(cl.remove(reg.clone()));
        assert!(cl.check("", "", "").is_empty());

        // Second removal returns false
        assert!(!cl.remove(reg));
    }

    #[test]
    fn test_resource_list_add_and_update() {
        let mut rl = ResourceList::new(std::sync::Weak::new());
        assert!(rl.check("", "", "").is_empty());

        let res1 = CrewResource {
            username: "user_a".to_string(),
            aliasname: "alias_a".to_string(),
            devicename: "dev_a".to_string(),
            addr: "10.1.1.1".to_string(),
            compiler_versions: vec![
                CompilerVersion {
                    version: "19.35".to_string(),
                    host: Arch::x64,
                    target: Arch::x64,
                }
            ],
        };

        // First update adds because not present
        rl.update(res1.clone());
        let all = rl.check("", "", "");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].aliasname, "alias_a");

        // Second update with same addr, devicename, username modifies aliasname & compilers
        let updated_res1 = CrewResource {
            username: "user_a".to_string(),
            aliasname: "new_alias".to_string(),
            devicename: "dev_a".to_string(),
            addr: "10.1.1.1".to_string(),
            compiler_versions: vec![
                CompilerVersion {
                    version: "19.40".to_string(),
                    host: Arch::x64,
                    target: Arch::arm64,
                }
            ],
        };
        rl.update(updated_res1);

        let all_after = rl.check("", "", "");
        assert_eq!(all_after.len(), 1);
        assert_eq!(all_after[0].aliasname, "new_alias");
        assert_eq!(all_after[0].compiler_versions.len(), 1);
        assert_eq!(all_after[0].compiler_versions[0].version, "19.40");
        assert_eq!(all_after[0].compiler_versions[0].target, Arch::arm64);

        // Update with different devicename adds new entry
        let res2 = CrewResource {
            username: "user_a".to_string(),
            aliasname: "alias_b".to_string(),
            devicename: "dev_b".to_string(),
            addr: "10.1.1.1".to_string(),
            compiler_versions: vec![],
        };
        rl.update(res2);
        assert_eq!(rl.check("", "", "").len(), 2);
    }

    #[test]
    fn test_resource_list_remove() {
        let mut rl = ResourceList::new(std::sync::Weak::new());
        let res = CrewResource {
            username: "u1".to_string(),
            aliasname: "a1".to_string(),
            devicename: "d1".to_string(),
            addr: "10.0.0.1".to_string(),
            compiler_versions: vec![],
        };
        rl.add(res);
        assert_eq!(rl.check("", "", "").len(), 1);

        // Remove exact
        assert!(rl.remove("10.0.0.1", "u1", "d1"));
        assert!(rl.check("", "", "").is_empty());

        // Remove non-existent
        assert!(!rl.remove("10.0.0.1", "u1", "d1"));
    }

    #[test]
    fn test_resource_list_check_specific() {
        let mut rl = ResourceList::new(std::sync::Weak::new());
        let res = CrewResource {
            username: "dev_user".to_string(),
            aliasname: "alias1".to_string(),
            devicename: "pc1".to_string(),
            addr: "192.168.1.10".to_string(),
            compiler_versions: vec![],
        };
        rl.add(res.clone());

        // Exact match
        let found = rl.check("192.168.1.10", "dev_user", "pc1");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], res);

        // Mismatched addr
        assert!(rl.check("192.168.1.99", "dev_user", "pc1").is_empty());
        // Mismatched username
        assert!(rl.check("192.168.1.10", "other_user", "pc1").is_empty());
        // Mismatched devicename
        assert!(rl.check("192.168.1.10", "dev_user", "pc2").is_empty());
    }


    #[test]
    fn test_arch_from_valid() {
        assert_eq!(Arch::from(0), Arch::unknown);
        assert_eq!(Arch::from(1), Arch::arm);
        assert_eq!(Arch::from(2), Arch::arm64);
        assert_eq!(Arch::from(3), Arch::x86);
        assert_eq!(Arch::from(4), Arch::x64);
    }

    #[test]
    #[should_panic(expected = "Unknown value")]
    fn test_arch_from_invalid_panics() {
        let _ = Arch::from(5);
    }

    #[test]
    fn test_serde_task() {
        let task = Task {
            username: "tester".to_string(),
            devicename: "laptop".to_string(),
            addr: "127.0.0.1".to_string(),
            core: 8,
            frequency: vec![2800],
            memory: 16.0,
            running: 2,
            max: 64,
            online: true,
        };
        let json = serde_json::to_string(&task).expect("serialize task");
        let deserialized: Task = serde_json::from_str(&json).expect("deserialize task");
        assert_eq!(task, deserialized);
    }

    #[test]
    fn test_serde_crew_register() {
        let json = r#"{
            "username": "worker1",
            "devicename": "node-01",
            "addr": "192.168.1.100",
            "passcode": "pass",
            "core": 4,
            "frequency": [2000, 2000],
            "memory": 8.0
        }"#;
        let reg: CrewRegister = serde_json::from_str(json).expect("deserialize register");
        assert_eq!(reg.username, "worker1");
        assert_eq!(reg.core, 4);
        assert_eq!(reg.frequency, vec![2000, 2000]);
    }

    #[test]
    fn test_serde_crew_resource() {
        let resource = CrewResource {
            username: "worker2".to_string(),
            aliasname: "build-box".to_string(),
            devicename: "node-02".to_string(),
            addr: "192.168.1.101".to_string(),
            compiler_versions: vec![
                CompilerVersion {
                    version: "MSVC 19.38".to_string(),
                    host: Arch::x64,
                    target: Arch::x86,
                }
            ],
        };
        let json = serde_json::to_string(&resource).expect("serialize crew resource");
        let de: CrewResource = serde_json::from_str(&json).expect("deserialize crew resource");
        assert_eq!(resource, de);
    }
}