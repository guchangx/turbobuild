mod commands;
mod compileripc;
use std::io::BufRead;

fn main() -> std::process::ExitCode {
    let is_crew_running = get_system_mark();
    if is_crew_running {
        match fetch_and_dist_compiler_commands() {
            Ok(_) => {
                return std::process::ExitCode::SUCCESS;
            },
            Err(_) => {
            return std::process::ExitCode::FAILURE;
            },
        }
    }
    else {
        let commandline = std::env::args_os();
        let commands: Vec<std::ffi::OsString> = commandline.collect();
        local_direct(commands);
        return std::process::ExitCode::SUCCESS;
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

fn local_direct(commands: Vec<std::ffi::OsString>) {
    let mut child = std::process::Command::new(&commands[0])
        .args(&commands[1..])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute compile.");

    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => println!("{}", l),
                Err(e) => eprintln!("read stdout error: {:?}", e),
            }
        }
    }

    child.wait().expect("failed to execute compile.");
}

fn get_system_mark() -> bool {
    let name = "Local\\CrewRunningSystemMark";
    let name_utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let h_map_memory = windows_sys::Win32::System::Memory::OpenFileMappingW (
            windows_sys::Win32::System::Memory::FILE_MAP_ALL_ACCESS,
            windows_sys::Win32::Foundation::FALSE,
            name_utf16.as_ptr(),
        );

        if h_map_memory.is_null() {
            let error = windows_sys::Win32::Foundation::GetLastError();
            println!("OpenFileMappingW failed. error: {}", error);
            return false;
        }

        let view = windows_sys::Win32::System::Memory::MapViewOfFile(
            h_map_memory,
            windows_sys::Win32::System::Memory::FILE_MAP_ALL_ACCESS,
            0,
            0,
            1
        );
        if view.Value.is_null() {
            let error = windows_sys::Win32::Foundation::GetLastError();
            println!("MapViewOfFile failed. {}", error);
            windows_sys::Win32::Foundation::CloseHandle(h_map_memory);
            return false;
        }

        let max = 8usize;
        let bytes = std::slice::from_raw_parts(view.Value as *const u8, max);
        let s = String::from_utf8_lossy(&bytes[..max]).to_string();

        windows_sys::Win32::System::Memory::UnmapViewOfFile(view);
        windows_sys::Win32::Foundation::CloseHandle(h_map_memory);
        if s.is_empty() {
            return false;
        }
        else {
            return true;
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn check_system_mark_test() {
        let mark = crate::get_system_mark();
        println!("system mark: {}", mark);
    }   
}