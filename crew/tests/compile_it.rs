
use crew::compileripc::socket::Receiver;
use crew::enter::Common;
use crew::communicate::distributor::Distributor;
use crew::roster::crews::TasksManager;
use crew::roster::crews::ResourceList;


#[test]
fn compile_test() {
    //cargo test --package crew --test compile_it -- compile_test --show-output
    println!("run cross ipc compile integration test");

    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let tasks = TasksManager::new();
    let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(tasks));
    
    let roster = ResourceList::new();
    let arc_roster = std::sync::Arc::new(std::sync::Mutex::new(roster));


    let dist = std::sync::Arc::new(std::sync::Mutex::new(Distributor::new(arc_tasks, arc_roster)));
    let receiver = Receiver::new(weak_common, dist);
    println!("init crew in crew it");
    //std::thread::spawn(move || {receiver.init()});
    
    socket_send();
}

fn socket_send() {
    use std::io::Write;
    use std::io::Read;

    match std::net::TcpStream::connect("localhost:9301") {
        Ok(mut stream) => {

            let compiler_commands = vec![std::ffi::OsString::from("/test")];
            let commands: Vec<_> = compiler_commands.into_iter().map(|item| item.into_string().unwrap()).collect();
            let data = format!(
                r#"{{"compiler_path": "{}", "compiler_working_dir": "{}", "compiler_commands": {:?}, "build_and_compiler_type": "{}"}}"#,
                std::ffi::OsString::from(r#"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\cl.exe"#).to_string_lossy(),
                std::ffi::OsString::new().to_string_lossy(),
                commands,
                std::ffi::OsString::from("MSBuild").to_string_lossy()
            );

            println!("send to turbobuild: {}", data);

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