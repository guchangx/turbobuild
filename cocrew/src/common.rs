
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
    log::info!("init cocrew");

    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let receiver = crate::communicate::unpackager::FileReceiver::new(weak_common.clone());
    let receiver_ = receiver.clone();
    
    weak_common.upgrade().unwrap().lock().unwrap().file_receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(receiver)));
    
    log::info!("common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    runtime.spawn(async move {
        crate::compiler::msvc::redirect_stdout_log();
    });
    
    runtime.block_on(async move {
        receiver_.init().await;
    });
    
    log::info!("end cocrew");
}