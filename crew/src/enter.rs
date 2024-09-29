
#[derive(Default, Clone)]
pub struct Common {
    pub socket: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::compileripc::socket::Receiver>>>,
    pub compiler_env: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::platform::windows::WindowsCompilerEnv>>>,
    pub fingerprint: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::fingerprint::gather::SystemInfo>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            socket: None,
            compiler_env: None,
            fingerprint: None,
        };
        
        return common;
    }
}

pub fn init() {
    
    log::debug!("init");
    
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let socket = crate::compileripc::socket::Receiver::new(weak_common.clone());
    let socket_ = socket.clone();
    let handle = std::thread::spawn(move || {
        socket_.init();
    });
    
    let arc_crews = std::sync::Arc::new(std::sync::Mutex::new(socket));
    weak_common.upgrade().unwrap().lock().unwrap().socket = Some(arc_crews.clone());
    
    let compiler_env =  crate::platform::windows::WindowsCompilerEnv::default();
    let arc_compiler_env = std::sync::Arc::new(std::sync::Mutex::new(compiler_env));
    weak_common.upgrade().unwrap().lock().unwrap().compiler_env = Some(arc_compiler_env.clone());
    
    let fingerprint = crate::fingerprint::gather::SystemInfo::new();
    let arc_fingerprint = std::sync::Arc::new(std::sync::Mutex::new(fingerprint));
    weak_common.upgrade().unwrap().lock().unwrap().fingerprint = Some(arc_fingerprint.clone());
    
    

    println!("common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    handle.join().expect("run notification receiver failed");
}