
pub struct CompilerInput {
    pub project: std::ffi::OsString,
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub env_vars: std::collections::HashMap<String, String>,
    pub build_and_compiler_type: std::ffi::OsString,
}

//TODO: should fetch build index, but it is not ideal way to do it.

pub fn fetch_compiler_args_path_from_envs(environment: &std::collections::HashMap<String, String>) -> (Option<std::ffi::OsString>, Option<std::ffi::OsString>) {

    let mut project = None;
    if let Some(sln) = environment.get("VSTEL_SolutionPath") {
        std::path::PathBuf::from(sln).file_stem().map(|stem,| {
            project = Some(stem.to_owned());
        });
    }

    let mut compiler = String::new();
    if let Some(dir) = environment.get("VSAPPIDDIR") {
        compiler = dir.clone();
    }

    if let Some(edition) = environment.get("VSSKUEDITION") {
        compiler.find(edition).map(|pos| {
            compiler.truncate(pos + edition.len() + 1);
        });
    }

    let mut buildstates = std::collections::HashMap::with_capacity(6);
    if let Some(tlog) = environment.get("TRACKER_INTERMEDIATE") {
        let dir = std::path::PathBuf::from(tlog);
        std::fs::read_dir(&dir).unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().unwrap().is_file())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".lastbuildstate"))
            .for_each(|entry| {
                let path = entry.path();
                let file = dir.join(path.file_name().unwrap());
                
                if let Ok(content) = std::fs::read_to_string(&*file) {

                    if let Some(line) = content.lines().next() {
                        line.split(':').for_each(|part| {
                            part.split_once('=').map(|(key, value)| {
                                buildstates.insert(key.to_string(), value.to_string());
                            });
                        });
                    }
                }
            });
    }

    if let Some(version) = buildstates.get("VCToolsVersion") {
        if !compiler.is_empty() {
            compiler.push_str(r#"VC\Tools\MSVC\"#);
            compiler.push_str(version);
        }
    }

    if let Some(arch) = buildstates.get("VCToolArchitecture") {
        if !compiler.is_empty() {
            if arch == "Native64Bit" {
                compiler.push_str(&format!(r#"\bin\Hostx64\{}\cl.exe"#, "x64"));
            }
            else if arch == "Native32Bit" {
                compiler.push_str(&format!(r#"\bin\Hostx64\{}\cl.exe"#, "x86"));
            }
        }
    }

    return (project, if compiler.is_empty() {None} else { Some(std::ffi::OsString::from(compiler))});
}

pub fn fetch_compiler_commands() -> Option<CompilerInput> {

    let mut environment = std::collections::HashMap::new();
    for (key, value) in std::env::vars() {
        environment.insert(key, value);
    }

    if let Some(project) = environment.get("VSTEL_MSBuildProjectFullPath") { 
        std::path::PathBuf::from(project).file_stem().map(|stem| {
            println!("assistbuild project: {:?} {:?}", stem, environment.get("VSTEL_ProjectID"));
        });
    }

    let (project_, compiler_) = fetch_compiler_args_path_from_envs(&environment);

    let commandline = std::env::args_os();
    let mut commands: Vec<std::ffi::OsString> = commandline.collect();
    let working_dir = std::env::current_dir().unwrap();

    if commands.len() <= 2  && commands.last().unwrap().to_string_lossy().ends_with(".rsp") {
        let (project, compiler, commands) = fetch_and_parse_commands_for_msbuild(&mut commands);
        match commands {
            Some(commands) => {

                let input = CompilerInput {
                    project: if project.is_empty() { project_.unwrap() } else { std::ffi::OsString::from(project) },
                    compiler_path: if compiler.is_empty() { compiler_.unwrap() } else { std::ffi::OsString::from(compiler) },
                    compiler_working_dir: std::ffi::OsString::from(working_dir),
                    compiler_commands: commands,
                    env_vars: environment,
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
            env_vars: environment,
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
        println!("{} read compiler args: {}", crate::compileripc::current_datetime(), commands);
        return Some(commands);
    }
    else {
        println!("{} {:?} .rsp file do not exist",  crate::compileripc::current_datetime(), compiler_response_file)   
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
                    
                    if !compiler.is_empty() {
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

    //TODO: project name mybe fetch form solution name.
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
            if count & 1 == 0 {
                if item.starts_with("\"") && item.ends_with("\"") {
                    let arg = item.strip_prefix("\"").unwrap().strip_suffix("\"").unwrap().to_owned();
                    result.push(arg);
                }
                else {
                    result.push(item.to_owned());    
                }
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
        if item.starts_with("/external:I") {
            if item == "/external:I" {
                if let Some(next) = iter.next() {
                    result_.push(std::ffi::OsString::from(item));
                    result_.push(std::ffi::OsString::from(next.trim_matches('"')));
                }
            }
            else {
                result_.push(std::ffi::OsString::from(item));
            }
        }
        else if item.starts_with("/I") || item.starts_with("/i") {

            if item == "/I" || item == "/i" {
                if let Some(next) = iter.next() {
                    //must be occupy two items
                    result_.push(std::ffi::OsString::from(item));
                    result_.push(std::ffi::OsString::from(next));
                }
            }
            else {

                let (left, right) = item.split_at("/I".len());

                result_.push(std::ffi::OsString::from(left));
                result_.push(std::ffi::OsString::from(right.trim_matches('"')));
            }
        }
        else if item.starts_with("/Fo") || item.starts_with("/Fd") {
             
            let arg_without_enclosed_quotation = item.replace('"', "").replace("\\\\", "\\");
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
        let line = r#"/c /IE:\TestFuture\GammaRay\GammaRayTool\build_enable\3rdparty\kde /Zi /nologo /W1 /WX- /diagnostics:column /Od /Ob0 /D _WINDLL /D UNICODE /D WIN32 /D UNICODE /D QT_CORE_LIB /D "CMAKE_INTDIR=\"Debug\"" /D MAKE_KITEMMODELS_LIB /Gm- /EHsc /RTC1 /MDd /GS /fp:precise /Zc:wchar_t /Zc:forScope /Zc:inline /GR /Fo"gammaray_kitemmodels.dir\Debug\\" /Fd"gammaray_kitemmodels.dir\Debug\vc143.pdb" /external:W0 /Gd /TP /wd4244 /errorReport:prompt BuildAssistCompilerPath:C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.39.33519\bin\Hostx64\x64\cl.exe BuildAssistProjectName:GammaRay /external:I "D:/WorkTool/Qt/qt_5.15.2.17/out64/include" /external:I "D:/WorkTool/Qt/qt_5.15.2.17/out64/include/QtCore" E:\TestFuture\GammaRay\GammaRayTool\3rdparty\kde\kmodelindexproxymapper.cpp E:\TestFuture\GammaRay\GammaRayTool\3rdparty\kde\krecursivefilterproxymodel.cpp"#;
        let (project, compiler, commands) = parse_commands_by_line(line);
        assert!(!project.is_empty() && compiler.ends_with("cl.exe"));
        println!("commands {:?}", commands);
    }

    #[test]
    fn parse_commands_include_space_path() {
        let line = r#"/c /I"E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\" /I "E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\" /Zi /nologo /Fo"gammaray kit.dir\Debug\\" /Fd"gammaray kit.dir\Debug\vc143.pdb""#;
        let (_project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:?}", commands);
        assert_eq!(commands.len(), 9);
    }

    #[test]
    fn parse_commands_path_with_double_qoutation() {
        let line = r#"/c /I"E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\" /I "E:\TestFuture\GammaRay\GammaRay Tool\3rdparty\" /Zi /nologo "E:\TestFuture\GammaRay\GammaRayTool\build_enable\launcher\win-injector\gammaray_wininjector_autogen\mocs_compilation_Debug.cpp""#;
        let (_project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:?}", commands);
        assert_eq!(commands.len(), 8);
    }


    #[test]
    fn parse_commands_other() {
        let line = r#"/c /I"D:\Webex\build_x64\spark-client-framework" /I"D:\Webex\spark-client-framework\." /I"D:\Webex\spark-client-framework\.." /I"D:\Webex\spark-client-framework\thirdparty\nlohmann\include" /Zi /W3 /WX /diagnostics:column /MP /O2 /Ob2 /Os /D UNICODE /D WIN32 /D NDEBUG /D _UNICODE /D bwc_EXPORTS /D CMAKE_BUILD /D "CMAKE_INTDIR=\"Release\"" /Gm- /EHsc /MD /GS /guard:cf /Gy /Qpar /fp:precise /Qspectre /Zc:wchar_t /Zc:forScope /Zc:inline /GR /std:c++17 /Fo"BwcCore.dir\Release\\" /Fd"BwcCore.dir\Release\BwcCore.pdb" /external:W3 /Gd /TP /errorReport:prompt /we4700 /Zc:__cplusplus /bigobj /F2000000 "D:\Webex\spark-client-framework\BroadWorksCalling\bwc\Source\boss_admin.cpp" "#;
        let (_project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:?}", commands);
    }

    #[test]
    fn parse_commands_with_external_args() {
        let line = r#"/c /I"D:\Webex\build_x64\spark-client-framework" /I"D:\Webex\spark-client-framework\." /I"D:\Webex\spark-client-framework\.." /I"D:\Webex\spark-client-framework\thirdparty\nlohmann\include" /external:I "D:/WorkTool/Qt/qt_5.15.2.17/out64/./mkspecs/win32-msvc" /Zi /W3 /WX /diagnostics:column /MP /O2 /Ob2 /Os /D UNICODE /D WIN32 /D NDEBUG /D "CMAKE_INTDIR="Release"" /Gm- /EHsc /MD /GS /guard:cf /Gy /Qpar /fp:precise /Qspectre /Zc:wchar_t /Zc:forScope /Zc:inline /GR /std:c++17 /Fo"BwcCore.dir\Release\\" /Fd"BwcCore.dir\Release\BwcCore.pdb" /external:W3 /Gd /TP /wd4251 /errorReport:prompt /we4700  /Zc:__cplusplus /bigobj /F2000000 "D:\Webex\spark-client-framework\BroadWorksCalling\bwc\Source\boss_admin.cpp" "#;
        let (_project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:#?}", commands);
    }

    #[test]
    fn parse_commands_with_define_string_value() {
        let line = r#"/c /I"D:\Webex\build_x64\spark-client-framework" /D "QT_TESTCASE_BUILDDIR="E:/TestFuture/GammaRay/GammaRayTool/build_enable"" "boss_admin.cpp" "#;
        let (_project, compiler, commands) = parse_commands_by_line(line);
        assert!(compiler.is_empty());
        println!("commands {:#?}", commands);
    }
    }