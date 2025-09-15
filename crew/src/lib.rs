pub mod communicate;
pub mod fingerprint;
pub mod compileripc;
pub mod enter;
pub mod platform;
pub mod compiler;
pub mod replica;
pub mod roster;
pub mod procemirror;

pub fn run() {
    //console_subscriber::init();
    
    tools::logger::init_logger("crew");

    let rt  = tokio::runtime::Builder::new_current_thread().thread_name("crew").enable_all().build().unwrap();
    let _ = rt.block_on(async move {
        enter::init().await;
    });
    
}