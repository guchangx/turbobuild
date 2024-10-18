
#[derive(serde::Deserialize)]
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

#[derive(serde::Deserialize)]
pub struct CrewRegister {
    pub username: String,
    pub devicename: String,
    pub aliasname: String,
    pub addr: String,
    pub password: String,
    pub license: String,
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
    
    pub fn check(&mut self, crew: CrewRegister) {
            
    }
    
    pub fn add(&mut self, crew: CrewConstitution) {
        self.crews.push(crew);
    }
    pub fn remove(&mut self, crew: CrewRegister) -> bool {
        let index = self.crews.iter().position(|arg| arg.username == crew.username && arg.aliasname == crew.aliasname && arg.addr == crew.addr);
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Version {
    pub version: String,
    pub host: Arch,
    pub target: Arch,
}

#[derive(Debug, Clone)]
pub struct CrewResource {
    pub username: String, 
    pub aliasname: String,
    pub addr: String,
    pub toolchains: Vec<Version>,
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

    pub fn check(&mut self, addr: &str, aliasname: &str, username: &str) -> Vec<CrewResource> {
        if addr.is_empty() && aliasname.is_empty() && username.is_empty() {
            let crews = self.crews.clone();
            return crews;
        }
        else {
            let mut resource = Vec::new();
            for crew in self.crews.clone() {
                if (!crew.addr.is_empty() && crew.addr == addr) 
                    && (crew.aliasname == aliasname)
                    && (!crew.username.is_empty() && crew.username == username) {
        
                        resource.push(crew);
                }
            }
            return resource
        }
    }
    
}
