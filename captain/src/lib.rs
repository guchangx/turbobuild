
extern crate prost;
extern crate tonic;
extern crate tokio;
extern crate hyper;
extern crate serde;

pub mod communicate;
pub mod roster;
pub mod common;

pub fn run() {
    tools::logger::init_logger("captain");
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_BACKTRACE", "full");
    let _ = common::init_common();
}