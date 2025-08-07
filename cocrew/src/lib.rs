pub mod communicate;
pub mod common;
mod compiler;
mod detours;
extern crate serde_json;

pub fn run() {    
    tools::logger::init_logger("cocrew");
    crate::common::init_common();
}