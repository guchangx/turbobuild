use sysinfo::RefreshKind;



#[derive(Default, Clone)]
pub struct Common {
    pub socket: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::compileripc::socket::Receiver>>>,
    pub compiler_env: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::platform::windows::WindowsCompilerEnv>>>,
    pub pool: std::option::Option<std::sync::Arc<tokio::runtime::Handle>>,
    pub roster: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>,
    pub tasks: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::TasksManager>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            socket: None,
            compiler_env: None,
            pool: None,
            roster: None,
            tasks: None,
        };
        
        return common;
    }
}

pub fn init() {
    
    log::debug!("init crew");

    let sys = sysinfo::System::new_with_specifics(RefreshKind::new().with_processes(sysinfo::ProcessRefreshKind::everything()),);
    
    let process = sys.processes_by_name("cocrew".as_ref());
    if process.count() >= 1 {
        println!("cocrew already running.");
    }
    else {
        std::process::Command::new("cocrew.exe")
            .spawn()
            .expect("failed to start cocrew");
    }
    
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let roster = crate::roster::crews::ResourceList::new();
    let arc_roster = std::sync::Arc::new(std::sync::Mutex::new(roster));

    let tasks = crate::roster::crews::TasksManager::new();
    let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(tasks));

    let dist = crate::communicate::distributor::Distributor::new(arc_tasks.clone(), arc_roster.clone());
    let arc_dist = std::sync::Arc::new(std::sync::Mutex::new(dist));

    weak_common.upgrade().unwrap().lock().unwrap().tasks = Some(arc_tasks);

    let socket = crate::compileripc::socket::Receiver::new(weak_common.clone(), arc_dist);
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

    runtime.spawn(crate::fingerprint::register::register_fingerprint_to_capation(runtime.handle().clone(), weak_common.clone()));
        
    weak_common.upgrade().unwrap().lock().unwrap().pool = Some(std::sync::Arc::new(runtime.handle().to_owned()));

    weak_common.upgrade().unwrap().lock().unwrap().roster = Some(arc_roster);

    println!("crew common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    handle.join().expect("run compiler ipc receiver failed");
    
}