pub mod communicate;
pub mod common;
mod compiler;
mod detours;
extern crate serde_json;

pub fn run() {
    //tracing_subscriber::fmt()
    //   .with_env_filter(tracing_subscriber::EnvFilter::new("tonic=trace,h2=trace,hyper=trace,tokio=info,cocrew=debug"))
    //   .with_thread_ids(true)
    //   .with_file(true)
    //   .with_line_number(true)
    //   .init();

    std::env::set_var("RUST_BACKTRACE", "1");
    std::panic::set_hook(Box::new(|info| 
        eprintln!("panic: {}", info))
    );

    tools::logger::init_logger("cocrew");
    crate::common::init_common();
}