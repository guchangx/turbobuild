use serde::de::value::SeqDeserializer;

static CONNECTED_ADDRS: std::sync::LazyLock<std::sync::Arc<tokio::sync::Mutex<Vec<String>>>> = std::sync::LazyLock::new(|| {
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
    pub async fn archive<'a>(addr: &str, path: &str, content: &std::borrow::Cow<'a, [u8]>, runtime: &std::sync::Arc<tokio::runtime::Handle>) -> String {
        let mut sender = super::package::Sender::new(addr, Some(runtime)).await;
        let file = super::package::ArchiveArgs {
            file_type: super::package::FileType::ToolChain,
            solution: "".to_string(),
            project: "".to_string(),
            name: "".to_string(),
            path:  path.to_string(),    
            content: content.clone(),
        };
        
        let file = super::package::SenderType::Archive(file);
        sender.dist(file).await;
        
        return "".to_string();
    }

    pub async fn archive_stream<'a>(addr: &str, runtime: &std::sync::Arc<tokio::runtime::Handle>) 
        -> (Option<tokio::sync::mpsc::Sender<super::package::ArchiveArgs<'static>>>, std::sync::Arc<tokio::sync::Notify>) {
        
        let (tx, rx) = tokio::sync::mpsc::channel::<super::package::ArchiveArgs>(128);

        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_ = notify.clone();
        let args = super::package::ArchiveStreamArgs {
            rx: rx,
            callback: Box::new(move || {
                notify.notify_waiters();
            })
        };

        let addr = addr.to_owned();
        let runtime_ = runtime.clone();
        runtime.spawn(async move {
            let mut sender = super::package::Sender::new(&addr, Some(&runtime_)).await;
            let archive = super::package::SenderType::ArchiveStream(args);
            sender.dist(archive).await;
        });
        
        return (Some(tx), notify_);
    }
    
    pub async fn compile<'a>(addr: &str, file: std::ffi::OsString, input: &crate::compiler::model::CompilerInput, content: &std::borrow::Cow<'a, [u8]>,
        runtime: &std::sync::Arc<tokio::runtime::Handle>) -> crate::communicate::package::ReceiverType {

        let args = super::package::SourcesFile {
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
            let mut addrs = CONNECTED_ADDRS.lock().await;
        
            if !addrs.iter().any(|item| item == addr) {
                let mut sender = super::package::Sender::new(addr, Some(runtime)).await;
                addrs.push(addr.to_string());
                runtime.spawn(async move {
                    sender.dist(super::package::SenderType::Command(crate::communicate::package::CommandArgs {})).await;
                });
            }
        }

        let mut sender = super::package::Sender::new(addr, Some(runtime)).await;
        
        let args = super::package::SenderType::Compile(args);
        let result = sender.dist(args).await;
        
        return result;
    }

    pub fn schedule(&self) -> String {
        let mut manager  = self.tasker.lock().unwrap();
        let addr = manager.schedule();
        return addr.to_owned();
    }
    
    pub fn schedule_for_sources(&self, addr: Option<String>, sources: &Vec<std::ffi::OsString>) -> (String, i32, Vec<std::ffi::OsString>, Vec<std::ffi::OsString>) {

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
            let (index, count) = manager.schedule_by_specific_host(addr.clone().unwrap().as_ref(), sources.len() as u32);
            
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
    
    pub fn done(&self, addr: &str) {
        let mut manager  = self.tasker.lock().unwrap();
        manager.done(addr);
    }

    pub fn all(&self) -> Vec<String> {
        let manager  = self.tasker.lock().unwrap();
        return manager.all().into_iter().map(|item| item.addr).collect();
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