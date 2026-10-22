use std::f32;

pub const STEP: u32 = 1;

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

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub cpu: f32,
    pub memory: f32,
}

fn default() -> bool {
    true
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Task {
    pub username: String,
    pub devicename: String,
    pub addr: String,
    pub core: u32,
    #[serde(default)]
    pub frequency: Vec<u64>,
    pub memory: f32,
    pub running: u32,
    pub max: u32,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default = "default")]
    pub online: bool,
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
    
    pub fn sync(&mut self, tasks: &Vec<Task>) {
        for task in tasks {
            let is_local = task.addr == "127.0.0.1" || task.addr == "localhost";
            if let Some(existing) = self.tasks.iter_mut().find(|t| {
                (is_local && (t.addr == "127.0.0.1" || t.addr == "localhost"))
                    || (!is_local && t.username == task.username && t.devicename == task.devicename && t.addr == task.addr)
            }) {
                existing.core = task.core;
                existing.memory = task.memory;
                existing.max = task.max;
                existing.addr = task.addr.clone();
                existing.frequency = task.frequency.clone();
                if !is_local {
                    existing.usage = task.usage.clone();
                }
                existing.online = task.online;
                if !existing.online {
                    existing.running = 0;
                }
            } else {
                self.tasks.push(task.clone());
            }
        }
    }

    pub fn add(&mut self, tasks: &Vec<Task>) {
        self.sync(tasks);
    }

    fn weight_for_frequency(frequency: u64) -> f64 {
        if frequency < 2000 {
            0.25
        }
        else if frequency < 4000 {
            0.5
        }
        else if frequency < 6000 {
            0.75
        }
        else {
            1.5
        }
    }

    pub fn schedule(&mut self) -> &str {
        let count = self.all().len();
        if count == 0 {
            return "";
        }
        else if count == 1 {
            if let Some(item) = self.tasks.iter_mut().find(|item| item.online) {
                item.running += 1;
                return item.addr.as_str();
            }
            return "";
        }
        else {
            let iter = self.tasks.iter_mut().filter(|item| item.online && item.running < item.max && item.usage.cpu <= 95.00).min_by(|x, y| {
                let xw = Self::weight_for_frequency(x.frequency.first().copied().unwrap_or(3000));
                let yw = Self::weight_for_frequency(y.frequency.first().copied().unwrap_or(3000));
                let x_ratio = x.running as f64 / (x.core as f64 * xw);
                let y_ratio = y.running as f64 / (y.core as f64 * yw);
                x_ratio.partial_cmp(&y_ratio).unwrap_or(std::cmp::Ordering::Equal)
            });
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
        let count = self.all().len();
        if count == 0 {
            return ("", -1, 0);
        }
        else if count == 1 {
            if let Some((idx, item)) = self.tasks.iter_mut().enumerate().find(|(_, item)| item.online) {
                item.running += size;
                return (item.addr.as_str(), idx as i32, size);
            }
            return ("", -1, 0);
        }
        else {
            let iter = self.tasks.iter_mut().enumerate().filter(|(_, item)| item.online && item.usage.cpu <= item.max as f32).min_by(|(_, x), (_, y)| {
                let xw = Self::weight_for_frequency(x.frequency.first().copied().unwrap_or(3000));
                let yw = Self::weight_for_frequency(y.frequency.first().copied().unwrap_or(3000));
                let x_ratio = x.running as f64 / (x.core as f64 * xw);
                let y_ratio = y.running as f64 / (y.core as f64 * yw);
                x_ratio.partial_cmp(&y_ratio).unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Some((idx, item)) = iter {
                let weight = Self::weight_for_frequency(item.frequency.first().copied().unwrap_or(3000));

                if size >= (item.core as f64 * weight) as u32 {
                    if size <= (item.core as f64 * (weight + 0.5)) as u32 {
                        item.running += size;
                        return (item.addr.as_str(), idx as i32, size);
                    }
                    else {
                        item.running += (item.core as f64 * weight) as u32;
                        return (item.addr.as_str(), idx as i32, (item.core as f64 * weight) as u32);
                    }
                }
                else {
                    item.running += size;
                    return (item.addr.as_str(), idx as i32, size);
                }            
            }
            else {
                return ("", -1, 0);
            }
        }
    }

    pub fn schedule_by_specific_host(&mut self, addr: &str, size: u32, index: Option<u32>) -> (i32, u32) {
        let online_count = self.tasks.iter().filter(|item| item.online).count();
        if online_count == 0 {
            return (-1, 0);
        }
        else if online_count == 1 {
            if let Some((idx, item)) = self.tasks.iter_mut().enumerate().find(|(_, item)| item.online) {
                item.running += size;
                return (idx as i32, size);
            }
            return (-1, 0);
        }
        else if index.is_some() {
            if self.can_run(addr) {
                if let Some((idx, item)) = self.tasks.iter_mut().enumerate().find(|(_, item)| item.addr == addr && item.online) {
                    item.running += STEP;
                    return (idx as i32, STEP);
                }
                else {
                    return (-1, 0);
                }
            }
            else {
                return (-1, 0);
            }
        }
        else {
            if let Some((idx, item)) = self.tasks.iter_mut().enumerate().find(|(_, item)| item.addr == addr && item.online) {
                
                let weight = Self::weight_for_frequency(item.frequency.first().copied().unwrap_or(3000));

                if size >= (item.core as f64 * weight) as u32  {
                    if size <= (item.core as f64 * (weight + 0.5)) as u32 {
                        item.running += size;
                        return (idx as i32, size as u32);
                    }
                    else {
                        item.running += (item.core as f64 * weight) as u32;
                        return (idx as i32, (item.core as f64 * weight) as u32);
                    }
                }
                else {
                    item.running += size;
                    return (idx as i32, size as u32);
                }
            }
            else {
                return (-1, 0);
            }
        }
    }
    
    pub fn done(&mut self, addr: &str, count: u32) {
        if let Some(task) = self.tasks.iter_mut().find(|item| item.addr == addr) {
            task.running = task.running.saturating_sub(count);
        }
    }

    pub fn remove(&mut self, tasks: &Vec<Task>) {
        for task in tasks {
            let is_local = task.addr == "127.0.0.1" || task.addr == "localhost";
            if let Some((idx, existing)) = self.tasks.iter_mut().enumerate().find(|(_, t)| {
                t.online && (
                    (is_local && (t.addr == "127.0.0.1" || t.addr == "localhost"))
                        || (!task.username.is_empty() && !task.devicename.is_empty() && t.username == task.username && t.devicename == task.devicename)
                        || (!task.addr.is_empty() && t.addr == task.addr)
                )
            }) {
                existing.online = false;
                existing.running = 0;
                log::info!("Crew node marked offline (slot {} preserved): {}@{}:{}", idx, existing.username, existing.devicename, existing.addr);
            }
        }
    }

    pub fn all(&self) -> Vec<Task> {
        let tasks = self.tasks.iter().filter(|item| item.online).cloned().collect();
        return tasks;
    }

    pub fn alls(&self) -> &Vec<Task> {
        &self.tasks
    }

    pub fn usage(&mut self, cpu: f32, memory: f32) {
        if let Some(task) = self.tasks.iter_mut().find(|item| (item.addr == "127.0.0.1" || item.addr == "localhost") && item.online) {
            task.usage.cpu = cpu;
            task.usage.memory = memory;
        }
    }

    pub fn can_run(&self, addr: &str) -> bool {
        if let Some(task) = self.tasks.iter().find(|item| item.addr == addr && item.online) {
            if task.running <= task.max {
                return true;
            }
            else {
                return false;
            }
        }
        else {
            return false;
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
            username: "".to_string(),
            devicename: "".to_string(),
            addr: "192.168.0.1".to_string(),
            core: 8,
            frequency: vec![1000],
            memory: 16.0,
            running: 0,
            max:2,
            usage: Usage {
                cpu: 0.0,
                memory: 0.0,
            },
            online: true,
        };
        tasks.add(&vec![task]);
        
        let task = Task {
            username: "".to_string(),
            devicename: "".to_string(),
            addr: "192.168.0.2".to_string(),
            core: 8,
            frequency: vec![1000],
            memory: 16.0,
            running: 0,
            max: 4,
            usage: Usage {
                cpu: 0.0,
                memory: 0.0,
            },
            online: true,
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

    #[test]
    fn test_schedule_for_sources_vector_index() {
        let mut tasks = TasksManager::new();
        let task0 = Task {
            username: "u0".to_string(),
            devicename: "d0".to_string(),
            addr: "192.168.0.10".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        let task1 = Task {
            username: "u1".to_string(),
            devicename: "d1".to_string(),
            addr: "192.168.0.11".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        tasks.add(&vec![task0.clone(), task1.clone()]);

        let (addr, idx, _count) = tasks.schedule_for_sources(1);
        assert_eq!(idx, 0);
        assert_eq!(addr, "192.168.0.10");

        // Mark node 0 offline
        tasks.remove(&vec![task0.clone()]);

        // Next schedule should pick node 1 at vector index 1
        let (addr, idx, _count) = tasks.schedule_for_sources(1);
        assert_eq!(idx, 1);
        assert_eq!(addr, "192.168.0.11");

        // Schedule by specific host for node 1 should return vector index 1
        let (idx, _count) = tasks.schedule_by_specific_host("192.168.0.11", 1, None);
        assert_eq!(idx, 1);

        // When only node 1 is online (online_count == 1), schedule_by_specific_host routes to the single online node (index 1)
        let (idx, _count) = tasks.schedule_by_specific_host("192.168.0.10", 1, None);
        assert_eq!(idx, 1);

        // When all nodes are offline (online_count == 0), should return (-1, 0)
        tasks.remove(&vec![task1.clone()]);
        let (idx, _count) = tasks.schedule_by_specific_host("192.168.0.10", 1, None);
        assert_eq!(idx, -1);
    }

    #[test]
    fn test_schedule_by_specific_host_online_count_one() {
        let mut tasks = TasksManager::new();
        let task0 = Task {
            username: "u0".to_string(),
            devicename: "d0".to_string(),
            addr: "192.168.0.10".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        let task1 = Task {
            username: "u1".to_string(),
            devicename: "d1".to_string(),
            addr: "192.168.0.11".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        tasks.add(&vec![task0.clone(), task1.clone()]);

        // When node 0 is marked offline, tasks.len() == 2, but online_count == 1
        tasks.remove(&vec![task0.clone()]);

        // schedule_by_specific_host should route directly to the single online node (node 1 at index 1)
        let (idx, size) = tasks.schedule_by_specific_host("192.168.0.11", 5, None);
        assert_eq!(idx, 1);
        assert_eq!(size, 5);

        // When all nodes offline, returns (-1, 0)
        tasks.remove(&vec![task1.clone()]);
        let (idx, size) = tasks.schedule_by_specific_host("192.168.0.11", 5, None);
        assert_eq!(idx, -1);
        assert_eq!(size, 0);
    }

    #[test]
    fn test_usage_matching() {
        let mut multi_tasks = TasksManager::new();
        let remote_task = Task {
            username: "remote_u".to_string(),
            devicename: "remote_d".to_string(),
            addr: "192.168.1.200".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        let local_task = Task {
            username: "local_u".to_string(),
            devicename: "local_d".to_string(),
            addr: "127.0.0.1".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        multi_tasks.add(&vec![remote_task, local_task]);

        multi_tasks.usage(88.0, 4096.0);
        let all_multi = multi_tasks.all();
        // remote node remains 0.0
        assert_eq!(all_multi[0].usage.cpu, 0.0);
        // local node (127.0.0.1) is updated to 88.0
        assert_eq!(all_multi[1].usage.cpu, 88.0);
        assert_eq!(all_multi[1].usage.memory, 4096.0);
    }

    #[test]
    fn test_remove_by_addr_and_vector() {
        let mut tasks = TasksManager::new();
        let t1 = Task {
            username: "user1".to_string(),
            devicename: "dev1".to_string(),
            addr: "192.168.1.50".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 2,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        let t2 = Task {
            username: "user2".to_string(),
            devicename: "dev2".to_string(),
            addr: "127.0.0.1".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 1,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        tasks.add(&vec![t1.clone(), t2.clone()]);
        assert_eq!(tasks.all().len(), 2);

        // Remove by Task vector
        tasks.remove(&vec![t1]);
        assert_eq!(tasks.all().len(), 1);
        assert_eq!(tasks.alls().len(), 2);
        assert!(!tasks.alls()[0].online);
        assert_eq!(tasks.alls()[0].running, 0);

        // Remove local by Task vector
        tasks.remove(&vec![t2]);
        assert_eq!(tasks.all().len(), 0);
        assert_eq!(tasks.alls().len(), 2);
        assert!(!tasks.alls()[1].online);
    }

    #[test]
    fn test_sync_preserves_local_usage_and_updates_state() {
        let mut tasks = TasksManager::new();
        let local = Task {
            username: "myuser".to_string(),
            devicename: "mydev".to_string(),
            addr: "127.0.0.1".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 0,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        tasks.add(&vec![local]);

        // Local task records real-time usage
        tasks.usage(45.5, 8192.0);
        assert_eq!(tasks.all()[0].usage.cpu, 45.5);

        // Captain sends full snapshot with usage default (0.0)
        let snapshot_task = Task {
            username: "myuser".to_string(),
            devicename: "mydev".to_string(),
            addr: "127.0.0.1".to_string(),
            core: 16, // upgraded cores
            frequency: vec![3200],
            memory: 32.0,
            running: 0,
            max: 20,
            usage: Usage::default(), // 0.0 from captain
            online: true,
        };
        tasks.sync(&vec![snapshot_task]);

        // Verify cores updated but local usage is preserved
        let all = tasks.all();
        assert_eq!(all[0].core, 16);
        assert_eq!(all[0].memory, 32.0);
        assert_eq!(all[0].usage.cpu, 45.5);
        assert_eq!(all[0].usage.memory, 8192.0);
    }
}


