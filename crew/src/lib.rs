pub mod communicate;
pub mod fingerprint;
pub mod compileripc;
pub mod enter;
pub mod platform;
pub mod compiler;
pub mod replica;
pub mod roster;
pub mod procemirror;


static ENFORCE_ACTIVATE_LOCAL_COCREW: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
    #[cfg(feature = "enforce_activate_local_cocrew")]
        return true;
    #[cfg(not(feature = "enforce_activate_local_cocrew"))]
        return false;
});

pub fn run() {
    //console_subscriber::init();
    
    tools::logger::init_logger("crew");

    let rt  = tokio::runtime::Builder::new_current_thread().thread_name("crew").enable_all().build().unwrap();
    let _ = rt.block_on(async move {
        enter::init().await;
    });
    
}