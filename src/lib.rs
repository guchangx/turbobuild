
#[macro_use]
extern crate async_trait;

extern crate redis;

mod network;
mod compiler;
mod utils;
mod platform;
mod cache;
mod buildturbo;
mod dist;
mod syncfile;
mod config;

pub fn main() {
    println!("welcome to build turbo tool.");
    let config = config::ConfigurationInfo::init();
    println!("config: {:?}", config);
    let working_parameters = buildturbo::WorkingParameters::init();
    network::server::NetworkRequestHandler::start(working_parameters);
    println!("end build turbo tool.");
}