
use std::io::Read;
use std::io::Write;

#[derive(Default, Clone)]
pub struct Receiver {
    port: u16,
    common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>,
    packager: std::sync::Arc<std::sync::Mutex::<crate::communicate::packager::Packager>>,
}

impl Receiver {
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>, packager: std::sync::Arc<std::sync::Mutex::<crate::communicate::packager::Packager>>) -> Self {
        return Self {
            port: 9301,
            common,
            packager,
        }
    } 
    
    pub fn init(&self) {
        println!("init ipc socket");
        let listener = std::net::TcpListener::bind(format!("localhost:{}", self.port)).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("new connection socket");
                    let runtime = self.common.upgrade().unwrap().lock().unwrap().pool.clone().unwrap();
                    let packager = self.packager.clone();
                    let runtime_ = runtime.clone();
                    let _ = runtime.spawn(async {
                        Self::handle_ipc_stream(stream, runtime_, packager).await;                        
                    });
                    //can't block current run.
                },
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        }
    }
    
    async fn handle_ipc_stream(mut stream: std::net::TcpStream, runtime: std::sync::Arc<tokio::runtime::Handle>, packager: std::sync::Arc<std::sync::Mutex::<crate::communicate::packager::Packager>>) {
        let mut buffer = [0 as u8; 512];
        
        loop {
            match stream.read(&mut buffer) {
                Ok(size) => {
                    println!("Received {} bytes", size);
                    println!("{}", String::from_utf8_lossy(&buffer[..size]));
                    
                    let input = crate::compiler::model::CompilerInput::default();
                    
                    crate::compiler::interface::request_compile(input, runtime.clone(), packager.clone()).await;
                    
                    stream.write_all(b"done").unwrap();
                },
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
    }
}