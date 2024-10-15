
extern crate prost;
extern crate tonic;
extern crate tokio;
extern crate hyper;
extern crate serde;

pub mod communicate;
pub mod roster;
pub mod common;

pub fn run() {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_BACKTRACE", "full");
    let _ = common::init_common();
}