
#[derive(Default, Clone)]
pub struct Distributor {
    restor: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::ResourceList>>,
}

impl Distributor {

    pub fn new(restor: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::ResourceList>>) -> Self {
        return Self{
            restor,
        }
    }

    
    pub fn sync<'a>(&self, path: &str, content: &std::borrow::Cow<'a, [u8]>) -> String {
        let mut restor = self.restor.lock().unwrap();
        
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

