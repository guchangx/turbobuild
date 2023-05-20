
#[macro_use]
extern crate async_trait;

extern crate redis;
extern crate env_logger;
extern crate serde;
extern crate serde_derive;

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
    init_logger();
    println!("welcome to build turbo.");
    let config = config::ConfigurationInfo::init();
    log::debug!("config: {:?}", config);
    let working_parameters = buildturbo::WorkingParameters::init(&config);
    network::server::NetworkRequestHandler::start(working_parameters);
    println!("build turbo exists.");
}

fn init_logger() {

    let mut builder = env_logger::Builder::new();
    let logger = builder
    .format_timestamp_millis()
    .write_style(env_logger::WriteStyle::Always)
    .format_level(true)
    .filter(Some("buildturbo"), log::LevelFilter::Trace)
    .target(env_logger::Target::Stdout)
    .try_init();

    match logger {
        Ok(_) => {
            println!("init logger sucessful");
        },
        Err(error) => {
            println!("init logger failed. {:?}", error);
        }
    }
}