
pub struct Crew {
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

pub struct List {
    crews: Vec<Crew>,
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
}

impl List {
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        return List {
            crews: Vec::new(),
            common,
        };
    }
    pub fn add(&mut self, crew: Crew) {
        self.crews.push(crew);
    }
    pub fn remove(&mut self, crew: Crew) -> bool {
        let index = self.crews.iter().position(|arg| arg.username == crew.username && arg.aliasname == crew.aliasname && arg.addr == crew.addr);
        match index {
            Some(index) => {
                self.crews.remove(index);
                true
            },
            None => false
        }
    }
    
    pub fn keepalive(&self, crew: Crew) {
        
    }
}