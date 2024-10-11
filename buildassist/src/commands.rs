
pub struct CompilerInput {
    pub compiler_path_or_arch: std::ffi::OsString,
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
                    compiler_path_or_arch: std::ffi::OsString::from(compiler),
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
            compiler_path_or_arch: compiler_path,
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
                    
                    let (compiler, commands) = parse_commands_line(line);
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

fn parse_compiler_commands(line: String) -> (String, Vec<std::ffi::OsString>) {

    //lead to package too large
    let mut args = Vec::new();
    let mut compiler = String::new();

    let mut line_ = line.clone();

    let assist = "AssistClCompilerPath:";
    if let Some(start) = line.find(assist) {
        let cl = "cl.exe";
        if let Some(end) = line[start..].find(cl) {
            let assit_cl = &line[start..(start + end + cl.len() + 1)];
            line_ = line.replace(assist, "").to_string();
            compiler = assit_cl.replace(assist, "");
        }
    }

    args = extract_macro_contain_space_arg(&mut line_);

    let mut args_with_path = extract_path_ecnclosed_quotation_arg(&mut line_);
    args.append(&mut args_with_path);

    let mut args_others = split_commonds_by_space(&mut line_);
    args.append(&mut args_others);

    if args.is_empty() {
        println!("compiler don't effective extract commands.")
    }

    /* 
    let winkits_includes = &working_compiler_env.winkits_includes_path;
    for include in winkits_includes {
        let mut instruct = "/I".to_string();
        instruct += include.to_str().unwrap();
        args.push(std::ffi::OsString::from(instruct));
    }

    let msvc_includes = working_compiler_env.msvc_includes_path.display().to_string();

    let mut instruct = String::from("/I");
    instruct += &msvc_includes;
    args.push(std::ffi::OsString::from(instruct));
    */
    return (compiler, args);
}

fn extract_macro_contain_space_arg(commands: &mut String) -> Vec<std::ffi::OsString> {
    use std::ops::Index;

    let mut args: Vec<std::ffi::OsString> = Vec::new();
    let replace_regex = regex::Regex::new(r#"(?P<a>/[Dd])(?P<b>[\s])(?P<c>"[\w=\\]+"[\w\\]+"")"#).unwrap();
    for capture in replace_regex.captures_iter(&commands.clone())
    {   
        let macro_definition = capture.index(0);
        let macro_definition_without_space = macro_definition.replace(" ", "");
        args.push(std::ffi::OsString::from(macro_definition_without_space));

        let mut macro_definitin_with_space = macro_definition.to_owned();
        macro_definitin_with_space.push_str(" ");

        let commands_ = commands.replace(macro_definitin_with_space.as_str(), "");
        *commands = String::from(commands_);
    }
    return args;
}

fn extract_path_ecnclosed_quotation_arg(compiler_commands: &mut String) -> Vec<std::ffi::OsString> {
    use std::ops::Index;
    let pathwithspace_regex = regex::Regex::new(r#"[-/\w:]*".*?""#).unwrap();
    let mut include_args: Vec<std::ffi::OsString> = Vec::new();

    for capture in pathwithspace_regex.captures_iter(&compiler_commands.clone())
    {
        if capture.len() == 1 {
            let path_contain_space = capture.index(0);

            let pathwithspace_position = compiler_commands.find(path_contain_space);
            match pathwithspace_position {
                Some(index) => {
                    let (head_content, _) = compiler_commands.split_at(index);
                    if head_content.len() > "external:I ".len() {
                        let prefix_index = head_content.len() - "external:I ".len();
                        let prefix_arg = head_content.to_owned().split_off(prefix_index);
                        if prefix_arg.eq("external:I ") {
                            let path_without_quotation = path_contain_space.replace('"', "");
                            let include_arg = "/I".to_string() + &path_without_quotation;
                            include_args.push(std::ffi::OsString::from(include_arg));

                        }
                    }
                },
                _ => {},
            }

            if path_contain_space.contains(" ") || 
                (path_contain_space.starts_with(r#"/I""#) && path_contain_space.ends_with(r#"""#)) ||
                path_contain_space.contains(".cpp") || path_contain_space.contains(".c") {
                let path_without_quotation = path_contain_space.replace('"', "");
                include_args.push(std::ffi::OsString::from(path_without_quotation));
                let arg = String::from(" ") + path_contain_space;
                *compiler_commands = String::from(compiler_commands.replace(arg.as_str(), ""));
            }
            else {

            }
        }
        else 
        {
            println!("regex capture len is not 1.");
        }
    }
    return include_args;
}

fn split_commonds_by_space(commands: &mut String) -> Vec<std::ffi::OsString> {

    let commands = commands.replace("  ", " ");
    let args_vec = commands.split(" ").map(|arg| String::from(arg)).collect::<Vec<_>>();
    let mut args: Vec<std::ffi::OsString> = Vec::new();

    for arg in args_vec {
        if arg.starts_with("/Fo") || arg.starts_with("/Fd") {
            let arg_without_enclosed_quotation = arg.replace('"', "");
            args.push(std::ffi::OsString::from(arg_without_enclosed_quotation));
        }
        else if !arg.is_empty() {
            args.push(std::ffi::OsString::from(arg));
        }
    }
    
    return args;
}

fn parse_commands_line(line: String) -> (String, Vec<std::ffi::OsString>) {

    let mut line_ = String::new();
    let mut compiler = String::new();
    let assist = "AssistClCompilerPath:";
    if let Some(start) = line.find(assist) {
        let cl = "cl.exe";
        if let Some(end) = line[start..].find(cl) {
            let assit_cl = &line[start..(start + end + cl.len() + 1)];
            line_ = line.replace(assist, "").to_string();
            compiler = assit_cl.replace(assist, "");
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