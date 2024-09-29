pub mod communicate;
pub mod fingerprint;
pub mod compileripc;
pub mod enter;
pub mod platform;
pub mod compiler;

pub fn run() {
    common::logger::init_logger();

    fingerprint::gather::SystemInfo::new();

    let rt  = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _ = rt.block_on(async move {
        enter::init();
    });
}