
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

    
    pub fn sync<'a>(&self, path: &str, content: &std::borrow::Cow<'a, [u8]>) -> String {
        let mut  = self.tasks_manager.lock().unwrap();
        
        for crew in restor.check("", "", "") {
            let mut sender = super::package::FileSender::new(crew.addr.as_str());
            
            let args = super::package::CommandArgs {
                
            };
            let command = super::package::SenderType::Command(args);
            sender.send(command);
        }
        
        return "".to_string();
    }

    fn scheduler() {
        
    }
}

