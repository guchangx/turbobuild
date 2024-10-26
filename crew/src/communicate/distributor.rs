
#[derive(Default, Clone)]
pub struct Distributor {
    tasks_manager: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::TasksManager>>,
}

impl Distributor {

    pub fn new(tasks_manager: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::TasksManager>>) -> Self {
        return Self{
            tasks_manager,
        }
    }
    
    pub async fn sync<'a>(addr: &str, path: &str, content: &std::borrow::Cow<'a, [u8]>) -> String {
        let mut sender = super::package::FileSender::new(addr);
        
        let file = super::package::ArchiveArgs {
            file_type: super::package::FileType::PrecompileedFile,
            name: "".to_string(),
            path: path.to_string(),
            content: content.clone(),
        };
        
        let file = super::package::SenderType::Archive(file);
        sender.send(file).await;
        
        return "".to_string();
    }

    pub fn schedule(&self) -> String{
        let mut manager  = self.tasks_manager.lock().unwrap();
        let addr = manager.schedule();
        println!("sync addr: {:?}", addr);
        return addr.to_owned();
    }
}

