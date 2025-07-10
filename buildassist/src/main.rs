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

    let handle = std::thread::spawn(||{
        compileripc::SocketClient::new()
    });

    let time = crate::compileripc::current_datetime();
    let mut project = String::new();
    #[allow(unused_assignments)]
    let mut result = Ok(());
    let input = commands::fetch_compiler_commands();
    match input {
        Some(input) => {
            project = format!("{:?} {}", input.project, input.index);
            println!("{} buildassist start: {:?}", time, project);
            match handle.join() {
                Ok(client) => {
                    if client.stream.is_some() {
                        match client.request_compile(&input) {
                            Ok(_) => {
                                result = Ok(());
                            },
                            Err(_) => {
                                local_retry(&input);
                            },
                        }
                    }
                    else {
                        local_retry(&input);
                    }
                },
                Err(_) => {
                    println!("buildassist thread join failed.");
                    return Err(());
                },
            }
        },
        None => {
            println!("buildassist fetch compiler commands failed.");
            result = Err(());
        },
    }
    println!("{} buildassist end: {} {:?} elapsed: {:?}.", crate::compileripc::current_datetime(), project, result, start.elapsed());
    return result;
}

fn local_retry(input: &commands::CompilerInput) {
    let exec = input.compiler_path.to_string_lossy().to_string();
    let working_dir = input.compiler_working_dir.to_string_lossy().to_string();
    let args: Vec<String> = input.compiler_commands.clone().into_iter()
        .map(|item| item.into_string().unwrap())
        .collect();

    let mut child = std::process::Command::new(exec)
        .args(args)
        .current_dir(working_dir)
        .spawn()
        .expect("failed to execute compile.");

    child.wait()
        .expect("failed to wait on child.");
}