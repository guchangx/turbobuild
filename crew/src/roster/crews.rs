
#[derive(Default, Debug, Clone)]
pub struct ResourceList {
    crews: Vec<crate::replica::toolchain::CrewsResource>,
}

impl ResourceList {
    pub fn new() -> Self {
        return ResourceList {
            crews: Vec::new()
        };
    }
    pub fn add(&mut self, crew: crate::replica::toolchain::CrewsResource) {
        self.crews.push(crew);
    }

    pub fn remove(&mut self, crew: crate::replica::toolchain::CrewsResource) -> bool {
        let index = self.crews.iter().position(|arg| arg.username == crew.username && arg.aliasname == crew.aliasname && arg.addr == crew.addr);
        match index {
            Some(index) => {
                self.crews.remove(index);
                true
            },
            None => false
        }
    }

    pub fn check(&mut self, addr: &str, username: &str, devicename: &str) -> Vec<crate::replica::toolchain::CrewsResource> {
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

    pub fn update(&mut self, crews: &Vec<crate::replica::toolchain::CrewsResource>) {

        //update
        for crew in self.crews.iter_mut() {
            
            let resource = crews.iter().find(|item| item.username == crew.username && item.devicename == crew.devicename);
            if let Some(resource) = resource {
                let mut resource = resource.compiler_versions.clone();
                crew.compiler_versions.clear();
                crew.compiler_versions.append(&mut resource); 
            }
        }

        //add
        for crew in crews {
            
            let exist = self.crews.iter().any(|item| item.username == crew.username && item.devicename == crew.devicename);
            if !exist {
                self.crews.push(crew.clone());
            }
        }
    }
    
}
