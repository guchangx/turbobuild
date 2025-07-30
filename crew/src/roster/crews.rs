use std::f32;


#[derive(Default, Clone)]
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
        log::debug!("add crew: {:?}", crew);
        self.crews.push(crew);
    }

    pub fn remove(&mut self, crew: crate::replica::toolchain::CrewsResource) -> bool {
        let index = self.crews.iter().position(|arg| arg.username == crew.username && arg.aliasname == crew.aliasname && arg.addr == crew.addr);
        match index {
            Some(index) => {
                log::debug!("remove crew: {:?}", crew);
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
                if (!crew.addr.is_empty() && crew.addr == addr) && (devicename.is_empty() || crew.devicename == devicename)
                    && (username.is_empty() || crew.username == username) {
                        resource.push(crew);
                }
            }
            return resource
        }
    }

    pub fn update(&mut self, crews: &Vec<crate::replica::toolchain::CrewsResource>) {

        log::debug!("update crews: {:?}", crews);
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

    pub fn choose() {
        
    }

    pub fn all(&self) -> Vec<crate::replica::toolchain::CrewsResource> {
        let crews = self.crews.clone();
        return crews;
    }
    
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct Usage {
    pub cpu: f32,
    pub memory: f32,
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct Task {
    #[serde(default)]
    pub index: i32,
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub core: u32,
    pub memory: f32,
    pub running: u32,
    pub max: u32,
    #[serde(default)]
    pub usage: Usage,
}

#[derive(serde::Deserialize, Default, Clone, Debug)]
pub struct TasksManager {
    tasks: Vec<Task>,
}

impl TasksManager {
    pub fn new() -> Self {
        return Self {
            tasks: Vec::new(),
        }
    }
    
    pub fn add(&mut self, tasks: &Vec<Task>) {
        for task in tasks {
            if let Some(existing) = self.tasks.iter_mut().find(|t| t.username == task.username && t.devicename == task.devicename && t.addr == task.addr) {
                existing.core = task.core;
                existing.memory = task.memory;
                existing.running = task.running;
                existing.max = task.max;
                existing.addr = task.addr.clone();
            } else {
                if task.addr == "127.0.0.1" || task.addr == "localhost" {
                    self.tasks.insert(0, task.clone());
                }
                else {
                    let mut task_ = task.clone();
                    task_.index = self.tasks.len() as i32;
                    self.tasks.push(task_);
                }
            }
        }
    }

    pub fn schedule(&mut self) -> &str {
        if self.tasks.len() == 1 {
            self.tasks.first_mut().unwrap().running += 1;
            return self.tasks.first().unwrap().addr.as_str();
        }
        else {
            let iter = self.tasks.iter_mut().filter(|item| item.running < item.max && item.usage.cpu <= 95.00).min_by(|x, y| x.running.cmp(&y.running));
            if let Some(item) = iter {
                item.running += 1;    
                return item.addr.as_str();
            }
            else {
                return "";    
            }            
        }
    }

    pub fn schedule_for_sources(&mut self, size: u32) ->(&str, i32, u32) {
        let iter = self.tasks.iter_mut().filter(|item| item.running < item.max).min_by(|x, y| x.running.cmp(&y.running));
        if let Some(item) = iter {
            if size <= item.core {
                if size <= size / 2 + item.core {
                    item.running += size;
                    return (item.addr.as_str(), item.index, size);
                }
                else {
                    item.running += item.core;
                    return (item.addr.as_str(), item.index, item.core);
                }
            }
            else {
                item.running += size;
                return (item.addr.as_str(), item.index, size);
            }            
        }
        else {
            return ("", -1, 0);
        }
    }

    pub fn schedule_by_specific_host(&mut self, addr: &str, size: u32) -> (i32, u32) {
        if let Some(item) = self.tasks.iter_mut().find(|item| item.addr == addr) {
            if size <= item.core {
                if size <= size / 2 + item.core {
                    item.running += size;
                    return (item.index,size as u32);
                }
                else {
                    item.running += item.core;
                    return (item.index, item.core as u32);
                }
            }
            else {
                item.running += size;
                return (item.index, size as u32);
            }
        }
        else {
            return (-1, 0);
        }
    }
    
    pub fn done(&mut self, addr: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|item| item.addr == addr) {
            task.running -= 1;
        }
    }

    pub fn remove(&mut self, username: &str, devicename: &str) {
        self.tasks.retain(|item| !(item.username == username && item.devicename == devicename));
    }

    pub fn all(&self) -> Vec<Task> {
        let tasks = self.tasks.clone();
        return tasks;
    }

    pub fn usage(&mut self, cpu: f32, memory: f32) {
        if let Some(task) = self.tasks.iter_mut().find(|item| item.addr == "127.0.0.1") {
            task.usage.cpu = cpu;
            task.usage.memory = memory;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //cargo test --package crew --tests shchedule_tasks -- --show-output
    fn shchedule_tasks() {
        println!("test schedule task");
        let mut tasks = TasksManager::new();
        let task = Task {
            index: 0,
            username: "".to_string(),
            devicename: "".to_string(),
            addr: "192.168.0.1".to_string(),
            core: 8,
            memory: 16.0,
            running: 0,
            max:2,
            usage: Usage {
                cpu: 0.0,
                memory: 0.0,
            },
        };
        tasks.add(&vec![task]);
        
        let task = Task {
            index: 1,
            username: "".to_string(),
            devicename: "".to_string(),
            addr: "192.168.0.2".to_string(),
            core: 8,
            memory: 16.0,
            running: 0,
            max: 4,
            usage: Usage {
                cpu: 0.0,
                memory: 0.0,
            },
        };
        tasks.add(&vec![task]);
        
        let addr = tasks.schedule();
        assert_eq!(addr, "192.168.0.1");

        let addr = tasks.schedule();
        assert_eq!(addr, "192.168.0.2");
        
        let addr = tasks.schedule();
        assert_eq!(addr, "192.168.0.1");

        let addr = tasks.schedule();
        assert_eq!(addr, "192.168.0.2");

        let addr = tasks.schedule();
        assert_eq!(addr, "192.168.0.2");

        let addr = tasks.schedule();
        assert_eq!(addr, "192.168.0.2");

        let addr = tasks.schedule();
        assert_eq!(addr, "");
    }
}