

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
    
    pub async fn sync<'a>(addr: &str, path: &str, content: &std::borrow::Cow<'a, [u8]>) -> String {
        let mut sender = super::package::FileSender::new(addr);
        
        let file = super::package::ArchiveArgs {
            file_type: super::package::FileType::ToolChain,
            name: "".to_string(),
            path:  path.to_string(),    
            content: content.clone(),
        };
        
        let file = super::package::SenderType::Archive(file);
        sender.send(file).await;
        
        return "".to_string();
    }
    
    pub async fn compile<'a>(addr: &str, file: std::ffi::OsString, input: &crate::compiler::model::CompilerInput, content: &std::borrow::Cow<'a, [u8]>) -> crate::communicate::package::ReceiverType {

        let args = super::package::PrecompiledFile {
            project: input.project.to_string_lossy().to_string(),
            file: file.to_string_lossy().to_string(),
            compiler: input.compiler_path.to_string_lossy().to_string(),
            working_dir: input.compiler_working_dir.to_string_lossy().to_string(),
            variety: input.build_and_compiler_type.to_string_lossy().to_string(),
            commands: input.compiler_commands.iter().map(|item| item.clone().into_string().unwrap()).collect(),
            content: content.clone(),
        };

        let mut sender = super::package::FileSender::new(addr);
        
        let args = super::package::SenderType::Compile(args);
        
        let result = sender.send(args).await;
        return result;
    }

    pub fn schedule(&self) -> String {
        let mut manager  = self.tasker.lock().unwrap();
        let addr = manager.schedule();
        return addr.to_owned();
    }
    
    pub fn done(&self, addr: &str) {
        let mut manager  = self.tasker.lock().unwrap();
        manager.done(addr);
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

