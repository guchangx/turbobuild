
#[derive(serde::Deserialize, serde::Serialize, Clone)]
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

#[derive(serde::Deserialize, Clone)]
pub struct CrewRegister {
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub passcode: String,
    pub core: u32,
    pub memory: f32,
}

#[derive(serde::Deserialize, Clone, serde::Serialize)]
pub struct Task {
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub core: u32,
    pub memory: f32,
    pub running: u32,
    pub max: u32
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
        self.tasks.push(task);
    }

    pub fn add_from_crew(&mut self, crew: &CrewRegister) {

        let task = Task {
            username: crew.username.clone(),
            devicename: crew.devicename.clone(),
            addr: crew.addr.clone(),
            core: crew.core,
            memory: crew.memory,
            running: 0,
            max: 100,
        };
        self.add(task);
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
}

pub struct ConstitutionList {
    crews: Vec<CrewConstitution>,
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
    
    pub fn keepalive(&self, crew: CrewConstitution) {
        
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CompilerVersion {
    pub version: String,
    pub host: Arch,
    pub target: Arch,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

    pub fn remove(&mut self, crew: CrewResource) -> bool {
        let index = self.crews.iter().position(|arg| arg.username == crew.username && arg.aliasname == crew.aliasname && arg.addr == crew.addr);
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
