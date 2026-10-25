
pub static CONNECTED_ADDR_MAP_SENDER: std::sync::LazyLock<std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, super::packager::Sender>>>> = std::sync::LazyLock::new(|| {
    std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()))
});

#[derive(Default, Clone)]
pub struct Distributor {
    tasker: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::TasksManager>>,
    resources: std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>,
    /// Notified whenever running slots are released, wakes up `schedule_for_sources_wait`.
    relslot: std::sync::Arc<tokio::sync::Notify>,
}

/// Releases the reserved running slots of a node on drop. See `Distributor::slot_guard`.
pub struct SlotGuard {
    distributor: Distributor,
    addr: String,
    count: u32,
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        self.distributor.done(&self.addr, self.count);
    }
}

impl Distributor {

    pub fn new(manager: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::TasksManager>>, resources: std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>) -> Self {
        return Self{
            tasker: manager,
            resources: resources,
            relslot: std::sync::Arc::new(tokio::sync::Notify::new()),
        }
    }
    
    //should replace with archive_stream, after test.
    //TODO: content should use bytes::Bytes, or remove this function and only use archive_stream.
    pub async fn archive<'a>(addr: &str, path: &str, content: &std::borrow::Cow<'a, [u8]>, runtime: &std::sync::Arc<tokio::runtime::Handle>) -> String {
        let mut sender = super::packager::Sender::new(addr, Some(runtime)).await;
        let file = super::packager::ArchiveArgs {
            file_type: super::packager::FileType::ToolChain,
            solution: "".to_string(),
            project: "".to_string(),
            name: "".to_string(),
            path:  path.to_string(),    
            content: bytes::Bytes::copy_from_slice(content.as_ref()),
        };
        
        let file = super::packager::SenderType::Archive(file);
        sender.dist(file).await;
        
        return "".to_string();
    }

    pub async fn archive_stream<'a>(addr: &str, runtime: &std::sync::Arc<tokio::runtime::Handle>) 
        -> (Option<tokio::sync::mpsc::Sender<super::packager::ArchiveArgs>>, std::sync::Arc<tokio::sync::Notify>) {
        
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        if (addr == "127.0.0.1" || addr == "localhost") && !*crate::ENFORCE_ACTIVATE_LOCAL_COCREW {
            notify.notify_one();
            return (None, notify);
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<super::packager::ArchiveArgs>(256);

        let notify_ = notify.clone();
        let args = super::packager::ArchiveStreamArgs {
            rx: rx,
            callback: Box::new(move || {
                notify.notify_waiters();
            })
        };

        let addr = addr.to_owned();
        let runtime_ = runtime.clone();
        runtime.spawn(async move {
            let mut sender = super::packager::Sender::new(&addr, Some(&runtime_)).await;
            let archive = super::packager::SenderType::ArchiveStream(args);
            sender.dist(archive).await;
        });
        
        return (Some(tx), notify_);
    }
    
    pub async fn compile<'a>(addr: &str, presyncedfiles: Vec<String>, input: &crate::compiler::model::CompilerInput, file: String, content: &std::borrow::Cow<'a, [u8]>,
        runtime: &std::sync::Arc<tokio::runtime::Handle>, output_callback: crate::compiler::model::OutputCallback) -> crate::communicate::packager::ReceiverType {

        let args = super::packager::SourcesFile {
            solution: input.solution.to_string_lossy().to_string(),
            project: input.project.to_string_lossy().to_string(),
            file: file,
            compiler: input.compiler_path.to_string_lossy().to_string(),
            working_dir: input.compiler_working_dir.to_string_lossy().to_string(),
            variety: input.build_and_compiler_type.to_string_lossy().to_string(),
            commands: input.compiler_commands.iter().map(|item: &std::ffi::OsString| item.clone().into_string().unwrap()).collect(),
            content: content.clone(),
            envs: input.envs.iter()
                .map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string()))
                .collect::<std::collections::HashMap<String, String>>(),
            presyncfiles: presyncedfiles,
        };

        let mut sender = {
            let mut addrs = CONNECTED_ADDR_MAP_SENDER.lock().await;
            if let Some(existing_sender) = addrs.get(addr) {
                existing_sender.clone()
            } 
            else {
                let sender = super::packager::Sender::new(addr, Some(&runtime)).await;
                addrs.insert(addr.to_string(), sender.clone());
                let mut sender_ = sender.clone();
                runtime.spawn(async move {
                    sender_.dist(super::packager::SenderType::Command(crate::communicate::packager::CommandArgs {})).await;
                });
                sender
            }
        };
        
        let args = super::packager::SenderType::Compile(args, output_callback);
        let result = sender.dist(args).await;
        return result;
    }

    pub fn schedule(&self) -> String {
        let mut manager  = self.tasker.lock().unwrap();
        let addr = manager.schedule();
        return addr.to_owned();
    }
    
    pub fn schedule_for_sources(&self, addr: Option<String>, sources: &Vec<std::ffi::OsString>, icore: Option<u32>) -> (String, i32, Vec<std::ffi::OsString>, Vec<std::ffi::OsString>) {

        if addr.is_none() {
            let mut manager  = self.tasker.lock().unwrap();
            let (addr, index, count) = manager.schedule_for_sources(sources.len() as u32);

            if count <= sources.len() as u32 {
                let left = sources.iter().take(count as usize).cloned().collect::<Vec<_>>();
                let right = sources.iter().skip(count as usize).cloned().collect::<Vec<_>>();
                return (addr.to_string(), index, left, right);
            }
            else {
                return (addr.to_string(), index, sources.to_vec(), Vec::new());
            }
        }
        else {
            let mut manager  = self.tasker.lock().unwrap();
            let (index, count) = manager.schedule_by_specific_host(addr.clone().unwrap().as_ref(), sources.len() as u32, icore);
            
            if count <= sources.len() as u32 {
                let left = sources.iter().take(count as usize).cloned().collect::<Vec<_>>();
                let right = sources.iter().skip(count as usize).cloned().collect::<Vec<_>>();
                return (addr.unwrap().to_string(), index, left, right);
            }
            else {
                return (addr.unwrap().to_string(), index, sources.to_vec(), Vec::new());
            }
        }
    }
    
    pub async fn schedule_for_sources_wait(&self, sources: &Vec<std::ffi::OsString>) -> (String, i32, Vec<std::ffi::OsString>, Vec<std::ffi::OsString>) {

        const INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

        let mut waiting = false;
        loop {
            // Register interest before checking, so a `done` between the check and the await is not lost.
            let notified = self.relslot.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let result = {
                let mut manager = self.tasker.lock().unwrap();
                manager.try_schedule_for_sources(sources.len() as u32)
            };

            match result {
                crate::roster::crews::TryScheduleResult::Scheduled(addr, index, count) => {
                    if waiting {
                        log::debug!("remote node {} has free resources, schedule continue.", addr);
                    }
                    let take = (count as usize).min(sources.len());
                    let left = sources.iter().take(take).cloned().collect::<Vec<_>>();
                    let right = sources.iter().skip(take).cloned().collect::<Vec<_>>();
                    return (addr, index, left, right);
                },
                crate::roster::crews::TryScheduleResult::NoNode
                | crate::roster::crews::TryScheduleResult::Full => {},
            }

            if !waiting {
                log::debug!("all remote nodes reached max running tasks or no node online, waiting for free resources.");
                waiting = true;
            }

            tokio::select! {
                _ = &mut notified => {},
                _ = tokio::time::sleep(INTERVAL) => {},
            }
        }
    }

    pub fn done(&self, addr: &str, count: u32) {
        let mut manager  = self.tasker.lock().unwrap();
        manager.done(addr, count);
        drop(manager);
        self.relslot.notify_waiters();
    }

    pub fn slot(&self, addr: &str, count: u32) -> SlotGuard {
        SlotGuard { distributor: self.clone(), addr: addr.to_string(), count }
    }

    pub fn all(&self) -> Vec<(String, u32)> {
        let manager  = self.tasker.lock().unwrap();
        return manager.all().into_iter().map(|item| (item.addr, item.core)).collect();
    }

    pub fn check(&self, addr: &str, cversion: &crate::replica::toolchain::CompilerVersion) -> bool {
        let mut resources = self.resources.lock().unwrap();

        let res = resources.check(addr, "", "");
        if res.is_empty() {
            return false;
        }
        else {
            if res.first().unwrap().compiler_versions.iter().find(|item| item.version == cversion.version && item.host == cversion.host && item.target == cversion.target).is_some() {
                return true;                
            }
            else {
                return false;
            }   
        }
    }
}