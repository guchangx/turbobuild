

#[cfg(test)]
mod integration_tests {
    #[test]
    fn compile_one_sourcefile_test() {
        //cargo test --package it --test cross_net_compile -- integration_tests::compile_test --exact --show-output --color always --nocapture
        tools::logger::init_logger("");
        println!("run cross ipc compile integration test");

        let captain = std::thread::spawn(|| { 
            captain::run();
        });

        std::thread::sleep(std::time::Duration::from_millis(200));

        let cocrew = std::thread::spawn(|| {
            cocrew::run();
        });

        let crew = std::thread::spawn(|| {
            crew::run();
        });

        std::thread::sleep(std::time::Duration::from_millis(3000));
        
        println!("send compile socket in integration test");
        let _ret = socket_send_compile_one_sourcefile();

        //captain.join().unwrap();
        //cocrew.join().unwrap();
        //crew.join().unwrap();

    }

    fn socket_send_compile_one_sourcefile() -> bool {
        use std::io::Write;
        use std::io::Read;
    
        match std::net::TcpStream::connect("localhost:22403") {
            Ok(mut stream) => {
    
                let env = crew::platform::windows::WindowsCompilerEnv::default();

                let mut path = std::path::PathBuf::from(env.compiler_path);
                path.push(r"Hostx64\x64\cl.exe");
                let path = path.to_string_lossy().replace(r"\", r"\\");

                let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
                compiler_commands.push(std::ffi::OsString::from("/c"));
                compiler_commands.push(std::ffi::OsString::from("/nologo"));
                compiler_commands.push(std::ffi::OsString::from("/MD"));
                compiler_commands.push(std::ffi::OsString::from("/GS"));
                compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
                compiler_commands.push(std::ffi::OsString::from("/Gy"));
                compiler_commands.push(std::ffi::OsString::from("/Qpar"));
                compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
                compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
                compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
                compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
                compiler_commands.push(std::ffi::OsString::from("/GR"));
                compiler_commands.push(std::ffi::OsString::from("/TP"));
                compiler_commands.push(std::ffi::OsString::from("/Zi"));
                compiler_commands.push(std::ffi::OsString::from("/I"));
                compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));

                for sdk_include in env.winkits_includes_path {
                    compiler_commands.push(std::ffi::OsString::from("/I"));
                    compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
                }

                //compiler_commands.push(std::ffi::OsString::from("/Fdlz4.pdb"));
                compiler_commands.push(std::ffi::OsString::from("/Folz4.obj"));

                let current_crate_dir = env!("CARGO_MANIFEST_DIR");
                let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
                current_crate_dir.pop();
                let draft_dir = current_crate_dir.join("draft");
                
                compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));
                compiler_commands.push(std::ffi::OsString::from(format!(r#"{}\lz4.c"#, draft_dir.to_string_lossy())));

                let commands: Vec<_> = compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();
                let data = format!(
                    r#"{{"project": "{}", "compiler_path": "{}", "compiler_working_dir": {:?}, "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
                    std::ffi::OsString::from("draft").to_string_lossy(),
                    path,
                    current_crate_dir.join("draft").to_string_lossy(),
                    commands,
                    std::ffi::OsString::from("MSBuild").to_string_lossy()
                );
    
                println!("it send to turbobuild: {}", data);
    
                stream.write(data.as_bytes()).unwrap();
    
                let mut data = String::new();
                let mut buffer = [0 as u8; 256];
                let mut reply = String::new();
                loop {
                    match stream.read(&mut buffer) {
                        Ok(size) => {
                            data = data + std::str::from_utf8(&buffer[0..size]).unwrap();
                            if size < buffer.len() {
                                reply = data;
                                break;
                            }
                        },
                        Err(err) => {
                            println!("it read from turbobuild server failed. {:?}", err);
                            break;
                        }
                    }
                }
                println!("replay: {}", reply);
                assert!(reply == "lz4.i");
                std::thread::sleep(std::time::Duration::from_secs(15));
                //std::process::exit(0);
            },
            Err(err) => {
                let kind = err.kind();
                let message = err.to_string();
                
                println!("buildassist failed: {:?}, {}", kind, message);
            },
        }
        return true;
    }

    #[test]
    fn compile_multi_sourcefile_test() {
        //cargo test --package it --test cross_net_compile -- integration_tests::compile_test --exact --show-output --color always --nocapture
        tools::logger::init_logger("");
        println!("run cross ipc compile integration test");

        let _captain = std::thread::spawn(|| { 
            captain::run();
        });

        std::thread::sleep(std::time::Duration::from_millis(200));

        let _cocrew = std::thread::spawn(|| {
            cocrew::run();
        });

        let _crew = std::thread::spawn(|| {
            crew::run();
        });

        std::thread::sleep(std::time::Duration::from_millis(3000));
        
        println!("send compile socket in integration test");
        let _ret = socket_send_compile_multi_source_file();

        //captain.join().unwrap();
        //cocrew.join().unwrap();
        //crew.join().unwrap();

    }

    fn socket_send_compile_multi_source_file() {
        use std::io::Write;
        use std::io::Read;
    
        match std::net::TcpStream::connect("localhost:22403") {
            Ok(mut stream) => {
    
                let env = crew::platform::windows::WindowsCompilerEnv::default();

                let mut path = std::path::PathBuf::from(env.compiler_path);
                path.push(r"Hostx64\x64\cl.exe");
                let path = path.to_string_lossy().replace(r"\", r"\\");

                let mut compiler_commands: Vec<std::ffi::OsString> = Vec::new();
                compiler_commands.push(std::ffi::OsString::from("/c"));
                compiler_commands.push(std::ffi::OsString::from("/nologo"));
                compiler_commands.push(std::ffi::OsString::from("/MD"));
                compiler_commands.push(std::ffi::OsString::from("/GS"));
                compiler_commands.push(std::ffi::OsString::from("/guard:cf"));
                compiler_commands.push(std::ffi::OsString::from("/Gy"));
                compiler_commands.push(std::ffi::OsString::from("/Qpar"));
                compiler_commands.push(std::ffi::OsString::from("/fp:precise"));
                compiler_commands.push(std::ffi::OsString::from("/Qspectre"));
                compiler_commands.push(std::ffi::OsString::from("/Zc:wchar_t"));
                compiler_commands.push(std::ffi::OsString::from("/Zc:forScope"));
                compiler_commands.push(std::ffi::OsString::from("/GR"));
                compiler_commands.push(std::ffi::OsString::from("/TP"));
                compiler_commands.push(std::ffi::OsString::from("/Zi"));
                compiler_commands.push(std::ffi::OsString::from("/EHsc"));
                compiler_commands.push(std::ffi::OsString::from("/I"));
                compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));

                for sdk_include in env.winkits_includes_path {
                    compiler_commands.push(std::ffi::OsString::from("/I"));
                    compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
                }

                let current_crate_dir = env!("CARGO_MANIFEST_DIR");
                let mut current_crate_dir = std::path::PathBuf::from(current_crate_dir);
                current_crate_dir.pop();
                let draft_dir = current_crate_dir.join("draft");

                //Fo arg must be dir.
                compiler_commands.push(std::ffi::OsString::from(format!(r#"/Fo{}\"#, draft_dir.to_string_lossy())));
                compiler_commands.push(std::ffi::OsString::from(format!(r#"/I {}"#, draft_dir.to_string_lossy())));

                for i in 0..10 {
                    let path = format!(r#"{}\test_{}.cpp"#, draft_dir.to_string_lossy(), i);
                    let mut file = std::fs::File::create(&path).unwrap();

                    let content = format!(r#"
                        #include <iostream>
                        int main()
                        {{
                            std::cout << "Hello Work." << std::endl;
                            return 0;
                        }}
                    "#);
                    file.write_all(content.as_bytes()).unwrap();

                    compiler_commands.push(std::ffi::OsString::from(path));
                }

                let commands: Vec<_> = compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();
                let data = format!(
                    r#"{{"project": "{}", "compiler_path": "{}", "compiler_working_dir": {:?}, "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
                    std::ffi::OsString::from("draft").to_string_lossy(),
                    path,
                    current_crate_dir.join("draft").to_string_lossy(),
                    commands,
                    std::ffi::OsString::from("MSBuild").to_string_lossy()
                );
    
                println!("it send to turbobuild: {}", data);
    
                stream.write(data.as_bytes()).unwrap();
                let _ = stream.set_nodelay(true);
                let _ = stream.set_read_timeout(Some(std::time::Duration::new(15, 0)));

                let mut buffer = [0 as u8; 256];
                let mut replys = Vec::new();
                loop {
                    match stream.read( &mut buffer) {
                        Ok(size) => {
                            if size == 0 {
                                break;
                            }
                            let data = buffer[0..size].to_vec();
                            if size < buffer.len() {
                                let reply = std::str::from_utf8(&data).unwrap().to_string();
                                println!("replay: {}", reply);
                                replys.push(reply);
                            }
                        },
                        Err(err) => {
                            println!("it read from turbobuild server failed. {:?}", err);
                            break;
                        }
                    }
                }
                println!("replys: {:#?}", replys);
                assert!(replys == vec![
                    "test_0.i",
                    "test_1.i",
                    "test_2.i",
                    "test_3.i",
                    "test_4.i",
                    "test_5.i",
                    "test_6.i",
                    "test_7.i",
                    "test_8.i",
                    "test_9.i",
                    "Generating Code...",
                ]);
                
                assert!(replys.iter().find(|&item| item == "Generating Code...").is_some());

                //std::thread::sleep(std::time::Duration::from_secs(15));
                //std::process::exit(0);
            },
            Err(err) => {
                let kind = err.kind();
                let message = err.to_string();
                
                println!("buildassist failed: {:?}, {}", kind, message);
            },
        }
    }

}


