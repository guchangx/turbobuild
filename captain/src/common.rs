
#[derive(Default)]
pub struct Common {
    pub notification: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::communicate::receiver::NotificationReceiver>>>,
    pub roster: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::List>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            notification: None,
            roster: None,
        };
        
        return common;
    }
}

pub fn init_common() {
    
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let crews = crate::roster::crews::List::new(weak_common.clone());
    let arc_crews = std::sync::Arc::new(std::sync::Mutex::new(crews));
    weak_common.upgrade().unwrap().lock().unwrap().roster = Some(arc_crews.clone());

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