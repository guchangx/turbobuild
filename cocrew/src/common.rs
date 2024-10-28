
#[derive(Default, Clone)]
pub struct Common {
    pub file_receiver: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::communicate::unpackager::FileReceiver>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            file_receiver: None,
        };
        return common;
    }
}

pub fn init_common() {
    println!("init cocrew");
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let receiver = crate::communicate::unpackager::FileReceiver::new(weak_common.clone());
    let receiver_ = receiver.clone();
    
    let handle = std::thread::spawn(move || {
        receiver_.init();            
    });

    weak_common.upgrade().unwrap().lock().unwrap().file_receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(receiver)));

    handle.join().unwrap();
    println!("common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
}