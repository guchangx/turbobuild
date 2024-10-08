mod commands;
mod network;

fn main() {
    fetch_and_dist_compiler_commands();
}

fn fetch_and_dist_compiler_commands() {
    let start = std::time::Instant::now();
    let commands = commands::fetch_compiler_commands();
    match commands {
        Some(commands) => {
            let client = network::NetworkClient::new();
            client.request_compile(commands);
            //client.request_local_compile(commands);
        },
        None => {
            println!("fetch compiler commands failed.");
        },
    }
    println!("compile elapsed: {:?}.", start.elapsed());
}