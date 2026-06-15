
pub static CONNECTED_ADDRS: std::sync::LazyLock<std::sync::Arc<tokio::sync::Mutex<Vec<String>>>> = std::sync::LazyLock::new(|| {
    std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()))
});

#[derive(Default, Clone)]
pub struct Distributor {
    tasker: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::TasksManager>>,
    resources: std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>,
}

impl Distributor {

    pub fn new(manager: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::TasksManager>>, resources: std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>) -> Self {
        return Self{
            tasker: manager,
            resources: resources,
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
            return (None, notify);
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<super::packager::ArchiveArgs>(128);

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
    
    pub async fn compile<'a>(addr: &str, file: std::ffi::OsString, input: &crate::compiler::model::CompilerInput, content: &std::borrow::Cow<'a, [u8]>,
        runtime: &std::sync::Arc<tokio::runtime::Handle>, output_callback: crate::compiler::model::OutputCallback) -> crate::communicate::packager::ReceiverType {

        let args = super::packager::SourcesFile {
            solution: input.solution.to_string_lossy().to_string(),
            project: input.project.to_string_lossy().to_string(),
            file: file.to_string_lossy().to_string(),
            compiler: input.compiler_path.to_string_lossy().to_string(),
            working_dir: input.compiler_working_dir.to_string_lossy().to_string(),
            variety: input.build_and_compiler_type.to_string_lossy().to_string(),
            commands: input.compiler_commands.iter().map(|item: &std::ffi::OsString| item.clone().into_string().unwrap()).collect(),
            content: content.clone(),
            envs: input.envs.iter()
                .map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string()))
                .collect::<std::collections::HashMap<String, String>>(),
        };

        {
            //TODO: should check the connecting status, if the addr is in connected addrs.
            let mut addrs = CONNECTED_ADDRS.lock().await;
        
            if !addrs.iter().any(|item| item == addr) {
                let mut sender = super::packager::Sender::new(addr, Some(runtime)).await;
                addrs.push(addr.to_string());
                runtime.spawn(async move {
                    sender.dist(super::packager::SenderType::Command(crate::communicate::packager::CommandArgs {})).await;
                });
            }
        }

        let mut sender = super::packager::Sender::new(addr, Some(runtime)).await;
        
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
    
    pub fn done(&self, addr: &str, count: u32) {
        let mut manager  = self.tasker.lock().unwrap();
        manager.done(addr, count);
    }

    pub fn all(&self) -> Vec<(String, u32)> {
        let manager  = self.tasker.lock().unwrap();
        return manager.all().into_iter().map(|item| (item.addr, item.core + 2)).collect();
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