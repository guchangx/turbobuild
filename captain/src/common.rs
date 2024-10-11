
#[derive(Default)]
pub struct Common {
    pub notification: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::communicate::receiver::NotificationReceiver>>>,
    pub constitutions: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ConstitutionList>>>,
    pub resources: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            notification: None,
            constitutions: None,
            resources: None,
        };
        
        return common;
    }
}

pub fn init_common() {
    
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let constitutions = crate::roster::crews::ConstitutionList::new(weak_common.clone());
    let arc_constitutions = std::sync::Arc::new(std::sync::Mutex::new(constitutions));
    weak_common.upgrade().unwrap().lock().unwrap().constitutions = Some(arc_constitutions);
    
    let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
    let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
    weak_common.upgrade().unwrap().lock().unwrap().resources = Some(arc_resources.clone());
    
    let receiver = crate::communicate::receiver::NotificationReceiver::new(weak_common.clone());

    let receiver_ = receiver.clone();
    let handle = std::thread::spawn(move || {
        receiver_.init();
    });
    
    let arc_notification_receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
    weak_common.upgrade().unwrap().lock().unwrap().notification = Some(arc_notification_receiver.clone());
    
    println!("common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    handle.join().expect("run notification receiver failed");
}