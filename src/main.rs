
#[macro_use]
extern crate lazy_static;
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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    println!("welcome to build turbo tool.");
    let working_parameters = buildturbo::WorkingParameters::init().await;
    network::server::NetworkRequestHandler::start(working_parameters).await;
    println!("end build turbo tool.");
}
