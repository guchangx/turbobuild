pub mod communicate;
pub mod common;
mod compiler;
pub mod detours;
extern crate serde_json;

pub fn run() {
    //console_subscriber::init();

    tools::logger::init_logger("cocrew");
    log::info!("Starting cocrew v{}", env!("CARGO_PKG_VERSION"));
    crate::common::init_common();
}
