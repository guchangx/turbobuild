
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

pub fn main() {
    println!("welcome to build turbo tool.");
    init_logger();
    let working_parameters = buildturbo::WorkingParameters::init();
    network::server::NetworkRequestHandler::start(working_parameters);
    println!("build turbo tool exists.");
}

fn init_logger() {

    let mut builder = env_logger::Builder::new();
    let logger = builder
    .write_style(env_logger::WriteStyle::Always)
    .format_level(true)
    .format_indent(Some(2))
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