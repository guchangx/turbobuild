use sysinfo::RefreshKind;

#[derive(Default, Clone)]
pub struct Common {
    pub socket: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::compileripc::socket::Receiver>>>,
    pub compiler_env: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::platform::windows::WindowsCompilerEnv>>>,
    pub pool: std::option::Option<std::sync::Arc<tokio::runtime::Handle>>,
    pub roster: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>,
    pub tasks: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::TasksManager>>>,
    pub notify: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::communicate::notifier::NotificationSender>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            socket: None,
            compiler_env: None,
            pool: None,
            roster: None,
            tasks: None,
            notify: None,
        };
        
        return common;
    }
}

pub fn run_cocrew() {
    log::debug!("check need init cocrew");
    
    let args = std::env::args().collect::<Vec<String>>();

    let mut iter = args.iter().skip_while(|item|!(item.starts_with("-noco") || item.starts_with("/noco")));
    
    if iter.next().is_some() {
        log::debug!("use nocorew, don't run cocrew in local.");
    }
    else {
        let sysinfo = sysinfo::System::new_with_specifics(RefreshKind::new().with_processes(sysinfo::ProcessRefreshKind::everything()));
        
        let process = sysinfo.processes_by_name("cocrew".as_ref());
        if process.count() >= 1 {
            log::debug!("cocrew already running.");
        }
        else {
            match std::process::Command::new("cocrew").spawn() {
                Ok(_) => {},
                Err(err) => {
                    log::error!("can't run cocrew error: {}", err);
                }
            }
        }   
    }
}

pub async fn init() {
    log::debug!("init crew");
    
    run_cocrew();
    
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let tasks = crate::roster::crews::TasksManager::new();
    let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(tasks));
    
    let roster = crate::roster::crews::ResourceList::new();
    let arc_roster = std::sync::Arc::new(std::sync::Mutex::new(roster));

    let dist = crate::communicate::distributor::Distributor::new(arc_tasks.clone(), arc_roster.clone());
    let arc_dist = std::sync::Arc::new(std::sync::Mutex::new(dist));

    weak_common.upgrade().unwrap().lock().unwrap().tasks = Some(arc_tasks);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name_fn(|| {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            format!("crew-worker-{}", id)
        })
        .build()
        .unwrap();

    let rt = runtime.handle();
    weak_common.upgrade().unwrap().lock().unwrap().pool = Some(std::sync::Arc::new(rt.clone()));

    let socket = crate::compileripc::socket::Receiver::new(weak_common.clone(), arc_dist);
    let socket_ = socket.clone();
    
    let handle = runtime.spawn(async move {
        socket_.init().await;
    });
    
    let arc_socket = std::sync::Arc::new(std::sync::Mutex::new(socket));
    weak_common.upgrade().unwrap().lock().unwrap().socket = Some(arc_socket.clone());
    
    let compiler_env =  crate::platform::windows::WindowsCompilerEnv::default();
    let arc_compiler_env = std::sync::Arc::new(std::sync::Mutex::new(compiler_env));
    weak_common.upgrade().unwrap().lock().unwrap().compiler_env = Some(arc_compiler_env.clone());
    
    let _ = runtime.spawn(crate::fingerprint::register::register_fingerprint_to_capation(rt.clone(), weak_common.clone()));

    weak_common.upgrade().unwrap().lock().unwrap().roster = Some(arc_roster);

    log::debug!("crew common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    
    let _ = handle.await; ("run compiler ipc receiver failed");
}