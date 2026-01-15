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
    use std::os::windows::process::CommandExt;

    let exec = &input.compiler_path;
    let working_dir = &input.compiler_working_dir;

    let mut process = std::process::Command::new(exec);
    for arg in &input.compiler_commands {
        process.raw_arg(arg);
    }

    let mut child = process
        .current_dir(working_dir)
        .spawn()
        .expect("failed to execute compile.");

    child.wait()
        .expect("failed to wait on child.");
}

fn get_system_mark() {
    let name = "Global\\BuildAssistSystemMark";
    let name_utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let h_map_memory = windows_sys::Win32::System::Memory::OpenFileMappingW (
            windows_sys::Win32::System::Memory::FILE_MAP_ALL_ACCESS,
            windows_sys::Win32::Foundation::FALSE,
            name_utf16.as_ptr(),
        );

        let shared_data = windows_sys::Win32::System::Memory::MapViewOfFile(
            h_map_memory,
            windows_sys::Win32::System::Memory::FILE_MAP_ALL_ACCESS,
            0,
            0,
            1
        );
    }
}