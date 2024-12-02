
#[cfg(test)]
mod it {
    #[test]
    fn compile_integrate_test() {
        //cargo test --package crew --test it_cross_net_compile -- compile_integrate_test --show-output --color always --nocapture
        println!("run cross ipc compile integration test");

        let captain = std::thread::spawn(|| { 
            captain::run();
        });

        std::thread::sleep(std::time::Duration::from_millis(200));

        let crew = std::thread::spawn(|| {
            crew::run();
        });

        std::thread::sleep(std::time::Duration::from_millis(200));
        
        println!("send compile socket in integration test");
        socket_send();

        captain.join().unwrap();
        crew.join().unwrap();
    }

    fn socket_send() {
        use std::io::Write;
        use std::io::Read;
    
        match std::net::TcpStream::connect("localhost:9301") {
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
                
                compiler_commands.push(std::ffi::OsString::from("/I"));
                compiler_commands.push(std::ffi::OsString::from(format!("{}", env.msvc_includes_path.to_str().unwrap())));

                for sdk_include in env.winkits_includes_path {
                    compiler_commands.push(std::ffi::OsString::from("/I"));
                    compiler_commands.push(std::ffi::OsString::from(format!("{}", sdk_include.to_str().unwrap())));
                }

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
                let mut buffer = [0 as u8; 128];
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
                            println!("read from turbobuild server failed. {:?}", err);
                            break;
                        }
                    }
                }
                assert!(reply == "done");
            },
            Err(err) => {
                let kind = err.kind();
                let message = err.to_string();
                
                println!("buildassist failed: {:?}, {}", kind, message);
            },
        }
    }
}


