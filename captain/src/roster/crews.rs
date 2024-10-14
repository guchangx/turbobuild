
pub struct CrewConstitution {
    pub username: String,
    pub aliasname: String,
    pub role: u32,
    pub addr: String,
    pub cores: u32,
    pub status: u32,
    pub action: u32,
    pub memory: u64,
    pub operating_system: String,
    pub cpu_frequency: f32,
    pub cpu_load: f32,
    pub description: String,
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
    pub fn add(&mut self, crew: CrewConstitution) {
        self.crews.push(crew);
    }
    pub fn remove(&mut self, crew: CrewConstitution) -> bool {
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
pub struct CrewResource {
    pub username: String, 
    pub aliasname: String,
    pub addr: String,
    pub winkits_includes_path: Vec<std::ffi::OsString>,
    pub compiler_path: std::ffi::OsString,
    pub msvc_includes_path: std::ffi::OsString,
    pub msvc_version: String,
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
