
pub struct CompilerInput {
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
}

pub fn fetch_compiler_commands() -> Option<CompilerInput> {
    let commandline =  std::env::args_os();
    let mut commands:Vec<std::ffi::OsString> = commandline.collect();
    
    let working_dir = std::env::current_dir().unwrap();
    if commands.len() <= 2  && commands.last().unwrap().to_string_lossy().ends_with(".rsp") {
        let (mut compiler, commands) = fetch_and_parse_commands_for_msbuild(&mut commands);
        match commands {
            Some(commands) => {

                println!("commands {:#?}", commands);
                if compiler.is_empty() {
                    compiler = "x64".to_string();
                }

                let input = CompilerInput {
                    compiler_path: std::ffi::OsString::from(compiler),
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
        let input = CompilerInput {
            compiler_path: compiler_path,
            compiler_working_dir: std::ffi::OsString::from(working_dir),
            compiler_commands: commands,
            build_and_compiler_type: std::ffi::OsString::from("CMake_MSVC"),
        };
        return Some(input);
     }
}

fn fetch_compiler_parameters_from_response_file(compiler_response_file: String) -> Option<String> {

    let response_file = std::path::Path::new(&compiler_response_file);
    if response_file.exists() {

        let contents = std::fs::read(response_file).unwrap();

        let mut index = 0 as usize;
        let mut commands = String::new();

        loop {
            let high = contents[index];
            let low = contents[index + 1];
            let v = u16::from_le_bytes([high, low]);
            if let Some(char)= std::char::from_u32(u32::from(v)) {
                if char != '\u{feff}' {
                    commands.push(char);
                }
            }
            
            index = index + 2;
            if index >= contents.len() {
                break;
            }
        }
        println!("read compiler args from temp .rsp file: {}", commands);
        return Some(commands);
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

fn fetch_and_parse_commands_for_msbuild(input_commands: &mut Vec<std::ffi::OsString>) -> (String, Option<Vec<std::ffi::OsString>>) {

    match input_commands.last() {
        Some(rspfile) => {

            match fetch_compiler_parameters_from_response_file(rspfile.to_string_lossy().replace("@", "")) {
                Some(line) => {
                    
                    let (compiler, commands) = parse_commands_by_line(line);
                    //let (compiler, commands) = parse_compiler_commands(line);
                    
                    let path = std::path::PathBuf::from(compiler.clone());
                    let _version = path.into_iter()
                    .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe")
                    .nth(0)
                    .unwrap();
                    
                    let _arch = path.into_iter()
                    .filter(|arg| arg.to_str().unwrap().starts_with("x86") || arg.to_str().unwrap().starts_with("x64"))
                    .nth(0)
                    .unwrap();

                    return (compiler, Some(commands));
                },
                None => {
                    return (String::new(), None);
                },
            };
        },
        None => {
            return (String::new(), None);
        }
    };
}

fn parse_commands_by_line(line: String) -> (String, Vec<std::ffi::OsString>) {

    let mut line_ = String::new();
    let mut compiler = String::new();
    let assist = "AssistClCompilerPath:";
    if let Some(start) = line.find(assist) {
        let cl = "cl.exe";
        if let Some(end) = line[start..].find(cl) {
            let assit_cl = &line[start..(start + end + cl.len() + 1)];
            line_ = line.replace(assist, "").to_string();
            compiler = assit_cl.replace(assist, "").trim_end().to_string();
        }
    }
    
    let commands:Vec<_> = line_.split(' ').collect();
    
    let mut result = Vec::new();
    let mut iter = commands.iter();

    for &item in iter.clone() {
        
        if item == "/D" || item == "/d" {
            if let Some(next) = iter.next() {
                result.push(std::ffi::OsString::from(item.to_owned() + " " + next));                
            }
        }
        // /I"E:\Test Future\GammaRay\GammaRayTool\build enable\3rdparty\kde"
        else if item.contains('"') {
            let count = item.matches('"').collect::<Vec<&str>>().len();
            if count % 2 == 0 {
                result.push(std::ffi::OsString::from(item));
            }
            else {

                let mut command = String::new();

                while let Some(next) = iter.next() {
                    if next.contains('"') {
                        result.push(std::ffi::OsString::from(command.clone() + item + " " + next));
                        break;
                    }
                    else {
                        command = command + item + " " + next + " ";
                    }
                }

                if let Some(next) = iter.next() {
                    if next.contains('"') {
                        result.push(std::ffi::OsString::from(item.to_owned() + " " + next));
                    }
                    else {
                        
                    }            
                }
            }
        }
        else if item.starts_with("/Fo") || item.starts_with("/Fd") {
            let arg_without_enclosed_quotation = item.replace('"', "");
            result.push(std::ffi::OsString::from(arg_without_enclosed_quotation));
        }
        else if item.is_empty() {
            iter.next();
        } 
        else {
            result.push(std::ffi::OsString::from(item));
            iter.next();
        }
    }
    
    return (compiler, result);
}