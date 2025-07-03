mod commands;
mod compileripc;

fn main() -> std::process::ExitCode {

    match fetch_and_dist_compiler_commands() {
        Ok(_) => {
            return std::process::ExitCode::SUCCESS;
        },
        Err(_) => {
           return std::process::ExitCode::FAILURE;
        },
    }
}

fn fetch_and_dist_compiler_commands() -> std::result::Result<(), ()> {
    let start = std::time::Instant::now();
    let time = crate::compileripc::current_datetime();
    let mut project = String::new();
    #[allow(unused_assignments)]
    let mut result = Ok(());
    let commands = commands::fetch_compiler_commands();
    match commands {
        Some(commands) => {
            project = format!("{:?} {:?}", commands.project, commands.index);
            println!("{} buildassist start: {:?}", time, project);
            let client = compileripc::SocketClient::new();
            match client.request_compile(commands) {
                Ok(_) => {
                   result = Ok(());
                },
                Err(_) => {
                    result = Err(());
                },
            }
        },
        None => {
            println!("buildassist fetch compiler commands failed.");
            result = Err(());
        },
    }
    println!("{} buildassist end: {:?} {:?} elapsed: {:?}.", crate::compileripc::current_datetime(), project, result, start.elapsed());
    return result;
}