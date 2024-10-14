
#[derive(Default, Clone)]
pub struct Common {
    pub socket: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::compileripc::socket::Receiver>>>,
    pub compiler_env: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::platform::windows::WindowsCompilerEnv>>>,
    pub pool: std::option::Option<std::sync::Arc<tokio::runtime::Handle>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            socket: None,
            compiler_env: None,
            pool: None,
        };
        
        return common;
    }
}

pub fn init() {
    
    log::debug!("init");
    
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let packager = crate::communicate::packager::Packager::default();
    
    let packager = std::sync::Arc::new(std::sync::Mutex::new(packager));
    let socket = crate::compileripc::socket::Receiver::new(weak_common.clone(), packager);
    let socket_ = socket.clone();
    
    let handle = std::thread::spawn(move || {
        socket_.init();
    });
    
    let arc_socket = std::sync::Arc::new(std::sync::Mutex::new(socket));
    weak_common.upgrade().unwrap().lock().unwrap().socket = Some(arc_socket.clone());
    
    let compiler_env =  crate::platform::windows::WindowsCompilerEnv::default();
    let arc_compiler_env = std::sync::Arc::new(std::sync::Mutex::new(compiler_env));
    weak_common.upgrade().unwrap().lock().unwrap().compiler_env = Some(arc_compiler_env.clone());
    
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    
    runtime.spawn(crate::fingerprint::register::register_fingerprint_to_capation(runtime.handle().clone()));
        
    weak_common.upgrade().unwrap().lock().unwrap().pool = Some(std::sync::Arc::new(runtime.handle().to_owned()));
    
    println!("common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    handle.join().expect("run compiler ipc receiver failed");
    
}