
pub struct CompilerInput {
    pub project: std::ffi::OsString,
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
        let (project, mut compiler, commands) = fetch_and_parse_commands_for_msbuild(&mut commands);
        match commands {
            Some(commands) => {

                println!("commands {:#?}", commands);
                if compiler.is_empty() {
                    compiler = "x64".to_string();
                }

                let input = CompilerInput {
                    project: std::ffi::OsString::from(project),
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
            project: std::ffi::OsString::from(""),
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

fn fetch_and_parse_commands_for_msbuild(input_commands: &mut Vec<std::ffi::OsString>) -> (String, String, Option<Vec<std::ffi::OsString>>) {

    match input_commands.last() {
        Some(rspfile) => {

            match fetch_compiler_parameters_from_response_file(rspfile.to_string_lossy().replace("@", "")) {
                Some(line) => {
                    
                    let (project, compiler, commands) = parse_commands_by_line(line.as_str());
                    
                    if compiler.is_empty() {
                        let path = std::path::PathBuf::from(compiler.clone());
                        let _version = path.into_iter()
                            .filter(|arg| arg.to_str().unwrap().contains(".") && arg.to_str().unwrap() != "cl.exe")
                            .nth(0)
                            .unwrap();
                        let _arch = path.into_iter()
                            .filter(|arg| arg.to_str().unwrap().starts_with("x86") || arg.to_str().unwrap().starts_with("x64"))
                            .nth(0)
                            .unwrap();
                    }
                    return (project, compiler, Some(commands));
                },
                None => {
                    return (String::new(), String::new(), None);
                },
            };
        },
        None => {
            return (String::new(), String::new(), None);
        }
    };
}

fn parse_commands_by_line(line: &str) -> (String, String, Vec<std::ffi::OsString>) {
    
    let mut line_ = String::new();
    let mut compiler = String::new();
    let assist = "BuildAssistCompilerPath:";
    if let Some(start) = line.find(assist) {
        let cl = "cl.exe";
        if let Some(end) = line[start..].find(cl) {
            let assit_cl = &line[start..(start + end + cl.len() + 1)];
            line_ = line.replace(assit_cl, "").to_string();
            compiler = assit_cl.replace(assist, "").trim_end().to_string();
        }
    }
    else {
        line_.push_str(line);
    }

    let mut line__ = String::new();
    let mut project = String::new();
    let assist = "BuildAssistProjectName:";
    if let Some(start) = line_.find(assist) {
        let delimiter = " ";
        if let Some(end) = line_[start..].find(delimiter) {
            let project_name = &line_[start..(start + end + delimiter.len())];
            line__ = line_.replace(project_name, "").to_string();
            project = project_name.replace(assist, "").trim_end().to_string();
        }
    }
    else {
        line__.push_str(&line_);
    }

    
    let commands: Vec<_> = line__.split(&[' ', '\u{A0}']).collect();

    let mut result = Vec::new();
    let mut iter = commands.iter();

    while let Some(&item) = iter.next() {
        if item.contains('"') {
            let count = item.matches('"').collect::<Vec<&str>>().len();
            if count % 2 == 0 {
                result.push(item.to_owned());
            }
            else {
                let mut command = String::new();

                while let Some(next) = iter.next() {
                    if next.contains('"') {
                        let arg = command.clone() + item + " " + next;
                        result.push(arg);
                        break;
                    }
                    else {
                        command = command + item + " " + next + " ";
                    }
                }
            }
        }
        else {
            result.push(item.to_owned());
        }
    }

    let mut result_ = Vec::new();
    let mut iter = result.iter();
    
    while let Some(item) = iter.next() {
        if item == "/D" || item == "/d" {
            if let Some(next) = iter.next() {
                let arg = std::ffi::OsString::from(item.to_owned() + " " + next);
                result_.push(arg);
            }
        }
        else if item.starts_with("/external:I") {
            if item == "/external:I" {
                if let Some(next) = iter.next() {
                    let arg = std::ffi::OsString::from(item.to_owned() + " " + next);
                    result_.push(arg);
                }
            }
            else {
                result_.push(std::ffi::OsString::from(item));
            }
        }
        else if item.starts_with("/i") || item.starts_with("/I") {

            if item == "/I" || item == "/i" {
                if let Some(next) = iter.next() {
                    let arg = std::ffi::OsString::from(item.to_owned() + " " + next);
                    result_.push(arg);
                }
            }
            else {
                result_.push(std::ffi::OsString::from(item));
            }
        }
        else if item.starts_with("/Fo") || item.starts_with("/Fd") {
            let arg_without_enclosed_quotation = item.replace('"', "");
            result_.push(std::ffi::OsString::from(arg_without_enclosed_quotation));
        }
        else if item.is_empty() {
            iter.next();
        }
        else {
            result_.push(std::ffi::OsString::from(item));
        }
    }
    
    return (project, compiler, result_);
}

/* 
fn parse_project_name_from_source_path(path: &String) -> std::ffi::OsString {
    let path = std::path::PathBuf::from(path);
    let mut path = path.canonicalize().unwrap();
    //E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\
    loop {
        path.pop();
        let mut has = false;

        path.push("CMakeLists.txt");
        if path.exists() {
            has = true;
        }
        else {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let metadata = entry.metadata().unwrap();
                        if metadata.is_file() {
                            if let Some(ext) = entry.path().extension() {
                                if ext == std::ffi::OsString::from(".sln") {
                                    has = true;
                                    break;
                                }
                            }                    
                        }
                    }
                }
            }
        }

        if has {
            if let Some(project) = path.components().last() {
                return project.as_os_str().to_os_string()
            }
        }
    }
}
*/
//TODO should fetch object name, but it is not ideal way to do it.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //cargo test --package buildassist --tests parse_commands -- --show-output
    fn parse_commands() {
        let line = r#"/c /IE:\TestFuture\GammaRay\GammaRayTool\build_enable\3rdparty\kde /Zi /nologo /W1 /WX- /diagnostics:column /Od /Ob0 /D _WINDLL /D _UNICODE /D UNICODE /D WIN32 /D _WINDOWS /D UNICODE /D _UNICODE /D _USING_V110_SDK71_=1 /D QT_DISABLE_DEPRECATED_BEFORE=0x050500 /D QT_USE_FAST_CONCATENATION /D QT_USE_FAST_OPERATOR_PLUS /D QT_NO_CAST_TO_ASCII /D QT_NO_URL_CAST_FROM_STRING /D QT_NO_DEBUG_OUTPUT /D QT_CORE_LIB /D "CMAKE_INTDIR=\"Debug\"" /D MAKE_KITEMMODELS_LIB /Gm- /EHsc /RTC1 /MDd /GS /fp:precise /Zc:wchar_t /Zc:forScope /Zc:inline /GR /Fo"gammaray_kitemmodels.dir\Debug\\" /Fd"gammaray_kitemmodels.dir\Debug\vc143.pdb" /external:W0 /Gd /TP /wd4244 /wd4267 /errorReport:prompt AssistClCompilerPath:C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.39.33519\bin\Hostx64\x64\cl.exe /external:I "D:/WorkTool/Qt/qt_5.15.2.17/out64/include" /external:I "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtCore" E:\TestFuture\GammaRay\GammaRayTool\3rdparty\kde\kmodelindexproxymapper.cpp E:\TestFuture\GammaRay\GammaRayTool\3rdparty\kde\krecursivefilterproxymodel.cpp"#;
        let (project, compiler, commands) = parse_commands_by_line(line);
        assert!(!project.is_empty() && compiler.ends_with("cl.exe"));
        println!("commands {:?}", commands);
    }

    #[test]
    fn parse_commands_include_space_path() {
        let line = r#"/c /I"E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\" /I "E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\" /Zi /nologo /Fo"gammaray kit.dir\Debug\\" /Fd"gammaray kit.dir\Debug\vc143.pdb""#;
        let (project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:?}", commands);
        assert_eq!(commands.len(), 7);
    }

    #[test]
    fn parse_commands_other() {
        let line = r#"/c /I"D:\Webex\build_x64\spark-client-framework" /I"D:\Webex\spark-client-framework\." /I"D:\Webex\spark-client-framework\.." /I"D:\Webex\spark-client-framework\thirdparty\nlohmann\include" /Zi /W3 /WX /diagnostics:column /MP /O2 /Ob2 /Os /D _UNICODE /D UNICODE /D WIN32 /D _WINDOWS /D NDEBUG /D TP_FOR_GENERIC=1 /D THREAD_SAFE_EVENTLOOP=1 /D UNICODE /D _UNICODE /D bwc_EXPORTS /D CMAKE_BUILD /D DESKTOP_PLATFORM /D SCF_STATIC_DEFINE /D "CMAKE_INTDIR=\"Release\"" /Gm- /EHsc /MD /GS /guard:cf /Gy /Qpar /fp:precise /Qspectre /Zc:wchar_t /Zc:forScope /Zc:inline /GR /std:c++17 /Fo"BwcCore.dir\Release\\" /Fd"BwcCore.dir\Release\BwcCore.pdb" /external:W3 /Gd /TP /wd4251 /wd4275 /errorReport:prompt /we4700 /we4701 /we6001 /we26494  /Zc:__cplusplus /bigobj /F2000000 "D:\Webex\spark-client-framework\BroadWorksCalling\bwc\Source\boss_admin.cpp" "#;
        let (project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:?}", commands);
    }
}