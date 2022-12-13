
#[macro_use]
extern crate async_trait;

extern crate redis;
extern crate env_logger;

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
    println!("welcome to build turbo tool.");
    let config = config::ConfigurationInfo::init();
    log::debug!("config: {:?}", config);
    let working_parameters = buildturbo::WorkingParameters::init(&config);
    network::server::NetworkRequestHandler::start(working_parameters);
    println!("build turbo tool exists.");
}

fn init_logger() {

    let mut builder = env_logger::Builder::new();
    let logger = builder
    .write_style(env_logger::WriteStyle::Always)
    .format_level(true)
    .filter(None, log::LevelFilter::Trace)
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