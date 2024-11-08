mod communicate;
mod common;
mod compiler;
mod detours;

pub fn run() {    
    tools::logger::init_logger();
    crate::common::init_common();
}