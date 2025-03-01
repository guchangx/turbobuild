pub mod communicate;
pub mod common;
mod compiler;
mod detours;

pub fn run() {    
    tools::logger::init_logger("cocrew");
    crate::common::init_common();
}