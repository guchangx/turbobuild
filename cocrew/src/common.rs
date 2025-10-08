
#[derive(Default, Clone)]
pub struct Common {
    pub receiver: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::communicate::unpackager::Receiver>>>, // receive data form coew include zip&command. 
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            receiver: None,
        };
        return common;
    }
}

pub static COCREW_RUNTIME: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex::<tokio::runtime::Runtime>>> = std::sync::LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all()
        .thread_name_fn(|| {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            format!("cocrew-worker-{}", id)
        }).build().unwrap();
    std::sync::Arc::new(std::sync::Mutex::new(runtime))
});

pub fn init_common() {
    log::info!("init cocrew");

    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    //TODO unpackager should rename grpc. crate::communicate::grpc::Receiver::new(weak_common.clone());
    let receiver = crate::communicate::unpackager::Receiver::new(weak_common.clone());
    let receiver_ = receiver.clone();
    
    weak_common.upgrade().unwrap().lock().unwrap().receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(receiver)));
    
    log::info!("common strang {} weak {}", weak_common.strong_count(), weak_common.weak_count());
    
    let handle = {
        let rt = COCREW_RUNTIME.lock().unwrap();
        rt.handle().clone()
    };
    
    //crate::communicate::syscallredirectpipe::compiler_redirect_syscall();
    crate::communicate::syscallredirectpipe::compiler_redirect_syscall_2();

    handle.spawn(async move {
        log::info!("start redirect_stdout_log_2_cocrew");
        crate::compiler::msvc::redirect_stdout_log();
    });
    
    let task = handle.spawn(async move {
        receiver_.init().await;
    });

    handle.block_on(async {
        let _ = task.await;
    });

    log::info!("end cocrew");
}