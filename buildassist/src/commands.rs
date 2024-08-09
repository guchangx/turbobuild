#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CompileInput {
    pub compiler_path_or_arch: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
}

pub fn fetch_compiler_commands() -> Option<CompileInput> {
    let commandline =  std::env::args_os();
    let mut commands:Vec<std::ffi::OsString> = commandline.collect();

    let working_dir = std::env::current_dir().unwrap();
    if commands.len() <= 2 {
        let commands = fetch_and_parse_commands_for_msbuild(&mut commands);
        match commands {
            Some(commands) => {
                let input = CompileInput {
                        compiler_path_or_arch: std::ffi::OsString::from("x64"),
                        compiler_working_dir: std::ffi::OsString::from(working_dir),
                        compiler_commands: commands,
                        build_and_compiler_type: std::ffi::OsString::from("MSBuild_MSVC"),
                    };
                return Some(input);
            },
            None => {
                return None;
            },
        }
     }
     else {
        let (compiler_path, commands) = fetch_and_parse_commands_for_cmake(&mut commands);
        let input = CompileInput {
            compiler_path_or_arch: compiler_path,
            compiler_working_dir: std::ffi::OsString::from(working_dir),
            compiler_commands: commands,
            build_and_compiler_type: std::ffi::OsString::from("Cmake_MSVC"),
        };
        return Some(input);
     }
}

fn fetch_compiler_parameters_from_response_file(compiler_response_file: String) -> Option<std::ffi::OsString> {

    let response_file = std::path::Path::new(&compiler_response_file);
    if response_file.exists() {

        let contents = std::fs::read(response_file).unwrap();

        let mut index = 0 as usize;
        let mut file_content = String::new();

        loop {
            let high = contents[index];
            let low = contents[index + 1];
            let v = u16::from_le_bytes([high, low]);
            if let Some(char)= std::char::from_u32(u32::from(v)) {
                if char != '\u{feff}' {
                    file_content.push(char);
                }
            }
            
            index = index + 2;
            if index >= contents.len() {
                break;
            }
        }
        println!("read compiler args from temp .rsp file: {}", file_content);
        return Some(std::ffi::OsString::from(file_content));
    }
    else {
        println!("{:?} .rsp file do not exist", compiler_response_file)   
    }
    return None;
}


fn fetch_and_parse_commands_for_cmake(input_commands: &mut Vec<std::ffi::OsString>) -> (std::ffi::OsString, Vec<std::ffi::OsString>) {
    input_commands.remove(0);
    let local_compiler_path = input_commands.remove(0);
    return (local_compiler_path, input_commands.clone());
}

fn fetch_and_parse_commands_for_msbuild(input_commands: &mut Vec<std::ffi::OsString>) -> Option<Vec<std::ffi::OsString>> {

    match input_commands.pop() {
        Some(rspfile) => {
            let commands_line = fetch_compiler_parameters_from_response_file(rspfile.to_string_lossy().replace("@", ""));
                match commands_line {
                Some(commands) => { 
                    return Some(vec![commands]);
                },
                None => {
                    return None;
                },
            };
        },
        None => {
            return None;
        }
    };
}